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
  non_jni_args: Punctuated<Ident, Comma>,
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
      if ret.to_token_stream().to_string() == "__VOID__" {
        None
      } else {
        Some(ret)
      }
    } else {
      None
    };

    let non_jni_args = if input.peek2(syn::token::Bracket) {
      let _: syn::Result<Comma> = input.parse();

      let non_jni_args;
      let _: syn::token::Bracket = bracketed!(non_jni_args in input);
      let non_jni_args = non_jni_args.parse_terminated::<Ident, Token![,]>(Ident::parse)?;
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
  let package = package
    .to_string()
    .replace("_", "_1")
    //  TODO: is this what we want? should we remove it instead?
    .replace("-", "_1");
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

  quote! {
    #[no_mangle]
    unsafe extern "C" fn #java_fn_name(
      env: JNIEnv,
      class: JClass,
      #(#args),*
    )  #ret {
      #function_before();
      #function(env, class, #(#args_),*, #(#non_jni_args),*)
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

#[proc_macro]
pub fn generate_package_name(tokens: TokenStream) -> TokenStream {
  let tokens = parse_macro_input!(tokens as GeneratePackageNameInput);
  let GeneratePackageNameInput { domain, package } = tokens;

  let domain = domain.to_string().replace("_", "/");
  let package = package
    .to_string()
    //  TODO: is this what we want? should we remove it instead?
    .replace("-", "_");

  let path = format!("{}/{}", domain, package);
  let litstr = LitStr::new(&path, proc_macro2::Span::call_site());

  quote! {#litstr}.into()
}
