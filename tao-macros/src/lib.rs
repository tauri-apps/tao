use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
  bracketed,
  parse::{Parse, ParseStream},
  parse_macro_input,
  punctuated::Punctuated,
  token::Comma,
  Ident, LitStr, Token, Type,
};

struct AndroidFnInput {
  domain: Ident,
  package: Ident,
  class: Ident,
  function: Ident,
  args: Punctuated<Type, Comma>,
  non_jni_args: Punctuated<Type, Comma>,
  ret: Option<Type>,
  function_before: Option<Ident>,
}

struct IdentArgPair(syn::Ident, syn::Type);

impl ToTokens for IdentArgPair {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let ident = &self.0;
    let type_ = &self.1;
    let tok = quote! {
      #ident: #type_
    };
    tokens.extend([tok]);
  }
}

impl Parse for AndroidFnInput {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let domain: Ident = input.parse()?;
    let _: Comma = input.parse()?;
    let package: Ident = input.parse()?;
    let _: Comma = input.parse()?;
    let class: Ident = input.parse()?;
    let _: Comma = input.parse()?;
    let function: Ident = input.parse()?;
    let _: Comma = input.parse()?;

    let args;
    let _: syn::token::Bracket = bracketed!(args in input);
    let args = args.parse_terminated::<Type, Token![,]>(Type::parse)?;
    let _: syn::Result<Comma> = input.parse();

    let ret = if input.peek(Ident) {
      let ret = input.parse::<Type>()?;
      let _: syn::Result<Comma> = input.parse();
      if ret.to_token_stream().to_string() == "__VOID__" {
        None
      } else {
        Some(ret)
      }
    } else {
      None
    };

    let non_jni_args = if input.peek(syn::token::Bracket) {
      let non_jni_args;
      let _: syn::token::Bracket = bracketed!(non_jni_args in input);
      let non_jni_args = non_jni_args.parse_terminated::<Type, Token![,]>(Type::parse)?;
      let _: syn::Result<Comma> = input.parse();
      non_jni_args
    } else {
      Punctuated::new()
    };

    let function_before = if input.peek(Ident) {
      let function: Ident = input.parse()?;
      let _: syn::Result<Comma> = input.parse();
      Some(function)
    } else {
      None
    };
    Ok(Self {
      domain,
      package,
      class,
      function,
      ret,
      args,
      non_jni_args,
      function_before,
    })
  }
}

/// Generates a JNI binding for a Rust function so you can use it as the extern for Java/Kotlin class methods in your android app.
///
/// This macro expects 5 mandatory parameters and 3 optional:
/// 1. snake_case representation of the reversed domain of the app. for example: com_tao
/// 2. snake_case representation of the package name. for example: tao_app
/// 3. Java/Kotlin class name.
/// 4. Rust function name (`ident`).
/// 5. List of extra types your Rust function expects. Pass empty array if the function doesn't need any arugments.
/// Note that all rust functions should expect the first two parameters to be [`JNIEnv`] and [`JClass`] so make sure they are imported into scope).
/// 6. (Optional) Return type of your rust function.
/// if you want to use the next parameter you need to provide a type or just pass `__VOID__` if the function doesn't return anything.
/// 7. (Optional) List of `ident`s to pass to the rust function when invoked (This mostly exists for internal usages).
/// 8. (Optional) Function to be invoked right before invoking the rust function (This mostly exists for internal usages).
///
/// ## Example
///
/// ```
/// # use tao_macros::android_fn;
/// # struct JNIEnv;
/// # struct JClass;
///
/// android_fn![com_example, tao, OperationsClass, add, [i32, i32], i32];
/// unsafe fn add(_env: JNIEnv, _class: JClass, a: i32, b: i32) -> i32 {
///   a + b
/// }
/// ```
/// which will expand into:
/// ```
/// # struct JNIEnv;
/// # struct JClass;
/// #[no_mangle]
/// unsafe extern "C" fn Java_com_example_tao_OperationsClass_add(
///   env: JNIEnv,
///   class: JClass,
///   a_1: i32,
///   a_2: i32
/// )  -> i32 {
///   add(env, class, a_1, a_2)
/// }
///
/// unsafe fn add(_env: JNIEnv, _class: JClass, a: i32, b: i32) -> i32 {
///   a + b
/// }
/// ```
/// and now you can extern the function in your Java/kotlin:
///
/// ```kotlin
/// class OperationsClass {
///   external fun add(a: Int, b: Int): Int;
/// }
/// ```
///
/// - [`JNIEnv`]: https://docs.rs/jni/latest/jni/struct.JNIEnv.html
/// - [`JClass`]: https://docs.rs/jni/latest/jni/objects/struct.JClass.html
#[proc_macro]
pub fn android_fn(tokens: TokenStream) -> TokenStream {
  let tokens = parse_macro_input!(tokens as AndroidFnInput);
  let AndroidFnInput {
    domain,
    package,
    class,
    function,
    ret,
    args,
    non_jni_args,
    function_before,
  } = tokens;

  let domain = domain.to_string();
  let package = package.to_string().replace("_", "_1").replace("-", "_1");
  let class = class.to_string();
  let args = args
    .into_iter()
    .enumerate()
    .map(|(i, t)| IdentArgPair(format_ident!("a_{}", i), t))
    .collect::<Vec<_>>();
  let non_jni_args = non_jni_args.into_iter().collect::<Vec<_>>();

  let java_fn_name = format_ident!(
    "Java_{domain}_{package}_{class}_{function}",
    domain = domain,
    package = package,
    class = class,
    function = function,
  );

  let args_ = args.iter().map(|a| &a.0);

  let ret = if let Some(ret) = ret {
    syn::ReturnType::Type(
      syn::token::RArrow(proc_macro2::Span::call_site()),
      Box::new(ret),
    )
  } else {
    syn::ReturnType::Default
  };

  let comma_before_non_jni_args = if non_jni_args.is_empty() {
    None
  } else {
    Some(syn::token::Comma(proc_macro2::Span::call_site()))
  };

  quote! {
    #[no_mangle]
    unsafe extern "C" fn #java_fn_name(
      env: JNIEnv,
      class: JClass,
      #(#args),*
    )  #ret {
      #function_before();
      #function(env, class, #(#args_),*  #comma_before_non_jni_args #(#non_jni_args),*)
    }

  }
  .into()
}

struct GeneratePackageNameInput {
  domain: Ident,
  package: Ident,
}

impl Parse for GeneratePackageNameInput {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let domain: Ident = input.parse()?;
    let _: Comma = input.parse()?;
    let package: Ident = input.parse()?;
    let _: syn::Result<Comma> = input.parse();

    Ok(Self { domain, package })
  }
}

/// Generate the package name used for invoking Java/Kotlin methods at compile-time
///
/// This macro expects 2 parameters:
/// 1. snake_case representation of the reversed domain of the app.
/// 2. snake_case representation of the package name.
///
/// ## Example
///
/// ```
/// # use tao_macros::generate_package_name;
///
/// const PACKAGE_NAME: &str = generate_package_name!(com_example, tao_app);
/// ```
/// which can be used later on with JNI:
/// ```
/// # use tao_macros::generate_package_name;
/// # fn find_my_class(env: i32, activity: i32, package: String) -> Result<(), ()> { Ok(()) }
/// # let env = 0;
/// # let activity = 0;
///
/// const PACKAGE_NAME: &str = generate_package_name!(com_example, tao_app); // constructs `com/example/tao_app`
/// let ipc_class = find_my_class(env, activity, format!("{}/Ipc", PACKAGE_NAME)).unwrap();
/// ```
#[proc_macro]
pub fn generate_package_name(tokens: TokenStream) -> TokenStream {
  let tokens = parse_macro_input!(tokens as GeneratePackageNameInput);
  let GeneratePackageNameInput { domain, package } = tokens;

  let domain = domain.to_string().replace("_", "/");
  let package = package.to_string().replace("-", "_");

  let path = format!("{}/{}", domain, package);
  let litstr = LitStr::new(&path, proc_macro2::Span::call_site());

  quote! {#litstr}.into()
}
