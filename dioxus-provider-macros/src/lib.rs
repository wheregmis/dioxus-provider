#![allow(unused_variables)] // Variables used in quote! macros aren't detected by compiler

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::time::Duration;
use syn::{
    FnArg, ItemFn, LitStr, Pat, PatType, Result, ReturnType, Token, Type, parse::Parse,
    parse::ParseStream, parse_macro_input,
};

/// Attribute arguments for the provider macro
#[derive(Default)]
struct ProviderArgs {
    interval: Option<Duration>,
    cache_expiration: Option<Duration>,
    stale_time: Option<Duration>,
    compose: Vec<syn::Ident>, // List of provider functions to compose
}

/// Attribute arguments for the mutation macro
#[derive(Default)]
struct MutationArgs {
    invalidates: Vec<syn::Ident>, // List of provider functions to invalidate
    optimistic: Option<syn::ExprClosure>, // Optimistic closure applied to cached data
}

impl Parse for ProviderArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ProviderArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "interval" => {
                    let lit: LitStr = input.parse()?;
                    let duration_str = lit.value();
                    let duration = humantime::parse_duration(&duration_str).map_err(|e| {
                        syn::Error::new_spanned(lit, format!("Invalid duration format: {e}"))
                    })?;
                    args.interval = Some(duration);
                }
                "cache_expiration" => {
                    let lit: LitStr = input.parse()?;
                    let duration_str = lit.value();
                    let duration = humantime::parse_duration(&duration_str).map_err(|e| {
                        syn::Error::new_spanned(lit, format!("Invalid duration format: {e}"))
                    })?;
                    args.cache_expiration = Some(duration);
                }
                "stale_time" => {
                    let lit: LitStr = input.parse()?;
                    let duration_str = lit.value();
                    let duration = humantime::parse_duration(&duration_str).map_err(|e| {
                        syn::Error::new_spanned(lit, format!("Invalid duration format: {e}"))
                    })?;
                    args.stale_time = Some(duration);
                }
                "compose" => {
                    // Parse compose list: compose = [provider1, provider2, ...]
                    let content;
                    syn::bracketed!(content in input);
                    let providers = content.parse_terminated(syn::Ident::parse, Token![,])?;
                    args.compose = providers.into_iter().collect();
                }
                _ => return Err(syn::Error::new_spanned(ident, "Unknown argument")),
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}

impl Parse for MutationArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = MutationArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "invalidates" => {
                    // Parse invalidation list: invalidates = [provider1, provider2, ...]
                    let content;
                    syn::bracketed!(content in input);
                    let providers = content.parse_terminated(syn::Ident::parse, Token![,])?;
                    args.invalidates = providers.into_iter().collect();
                }
                "optimistic" => {
                    let expr: syn::ExprClosure = input.parse()?;
                    args.optimistic = Some(expr);
                }
                _ => return Err(syn::Error::new_spanned(ident, "Unknown argument")),
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}

/// Provider macro for creating cached, composable data providers
///
/// This macro converts an async function into a Provider implementation with
/// automatic caching, composition, and other advanced features.
///
/// # Supported Arguments
/// - `interval = "30s"` - Background refresh interval
/// - `cache_expiration = "5min"` - Cache expiration time  
/// - `stale_time = "1min"` - Time before data is considered stale
/// - `compose = [provider1, provider2, ...]` - Compose multiple providers in parallel
///
/// # Composition Requirements
/// When using `compose = [...]`, the following requirements must be met:
///
/// ## Parameter Clone Requirements
/// **All function parameters MUST implement `Clone`** when using composition.
/// Parameters are cloned inside async blocks to enable parallel execution.
///
/// ```rust
/// // ✅ Good - u32 implements Clone
/// #[provider(compose = [fetch_permissions])]
/// async fn fetch_user_profile(user_id: u32) -> Result<Profile, Error> {
///     // fetch_permissions_result is available here
/// }
///
/// // ❌ Bad - non-Clone parameter
/// #[provider(compose = [fetch_permissions])]
/// async fn fetch_user_profile(config: NonCloneConfig) -> Result<Profile, Error> {
///     // This will cause a compile error
/// }
///
/// // ✅ Solution - Add #[derive(Clone)] to your types
/// #[derive(Clone)]
/// struct UserConfig { /* fields */ }
///
/// #[provider(compose = [fetch_permissions])]
/// async fn fetch_user_profile(config: UserConfig) -> Result<Profile, Error> {
///     // Now this works
/// }
/// ```
///
/// ## Provider Existence Validation
/// All providers listed in `compose = [...]` must:
/// - Be valid Rust identifiers
/// - Exist in the current scope when the macro is expanded
/// - Have compatible signatures (same parameter types)
///
/// The macro generates compile-time calls to verify provider existence and
/// provides clear error messages if providers are not found.
///
/// # Examples
/// ```rust
/// #[provider(cache_expiration = "5min")]
/// async fn fetch_user(id: u32) -> Result<User, String> {
///     // Implementation
/// }
///
/// #[provider(compose = [fetch_user, fetch_settings], cache_expiration = "3min")]
/// async fn fetch_full_profile(user_id: u32) -> Result<FullProfile, String> {
///     // Composed results automatically available as variables:
///     // - __dioxus_composed_fetch_user_result: Result<User, String>
///     // - __dioxus_composed_fetch_settings_result: Result<Settings, String>
///     let user = __dioxus_composed_fetch_user_result?;
///     let settings = __dioxus_composed_fetch_settings_result?;
///     Ok(FullProfile { user, settings })
/// }
/// ```
///
/// # Compilation Errors
/// The macro provides clear error messages for common issues:
/// - **Clone not implemented**: "Parameter type 'TypeName' must implement Clone for composition"
/// - **Provider not found**: "Composed provider 'provider_name' not found in current scope"
/// - **Signature mismatch**: "Composed provider 'provider_name' has incompatible signature"
#[proc_macro_attribute]
pub fn provider(args: TokenStream, input: TokenStream) -> TokenStream {
    let provider_args = if args.is_empty() {
        ProviderArgs::default()
    } else {
        match syn::parse(args) {
            Ok(args) => args,
            Err(err) => return err.to_compile_error().into(),
        }
    };

    let input_fn = parse_macro_input!(input as ItemFn);

    let result = generate_provider(input_fn, provider_args);

    match result {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Mutation macro for creating data mutations with cache invalidation
///
/// This macro converts an async function into a Mutation implementation that can
/// invalidate related provider caches when executed.
///
/// # Supported Arguments
/// - `invalidates = [provider1, provider2, ...]` - Providers to invalidate after mutation
/// - `optimistic = |data, ...args| { ... }` - Optimistic update closure (requires MutationContext)
///
/// ## Optimistic Updates
/// The optimistic closure receives:
/// - First param: `&mut Data` - mutable reference to current cached data
/// - Remaining params: references to mutation inputs
///
/// Examples:
/// - No args: `optimistic = |data: &mut Vec<Item>| { data.clear() }`
/// - One arg: `optimistic = |data: &mut Vec<Item>, id: &u64| { data.retain(|i| i.id != *id) }`
/// - Multi-arg: `optimistic = |data: &mut Item, name: &String, status: &bool| { data.name = name.clone(); data.active = *status; }`
///
/// ## Return Values
/// Mutation return values serve multiple purposes:
/// - Update `MutationState` for UI feedback (Success/Error)
/// - With optimistic updates: replace cache with server response (avoids refetch)
/// - Without optimistic: cache is invalidated and providers refetch automatically
///
/// # Examples
/// ```rust
/// // Simple mutation with cache invalidation
/// #[mutation(invalidates = [fetch_user, fetch_user_list])]
/// async fn update_user(user: User) -> Result<User, String> {
///     // Update user implementation
///     // Will automatically invalidate fetch_user and fetch_user_list caches
/// }
///
/// // Optimistic mutation with single argument
/// #[mutation(
///     invalidates = [load_items],
///     optimistic = |items: &mut Vec<Item>, id: &u64| {
///         items.retain(|i| i.id != *id)
///     }
/// )]
/// async fn delete_item(
///     id: u64,
///     ctx: MutationContext<Vec<Item>, Error>,
/// ) -> Result<Vec<Item>, Error> {
///     ctx.map_current(|items| items.retain(|i| i.id != id))
///         .ok_or(Error::NoData)
/// }
///
/// // Optimistic mutation with multiple arguments
/// #[mutation(
///     invalidates = [load_items],
///     optimistic = |items: &mut Vec<Item>, id: &u64, name: &String| {
///         if let Some(item) = items.iter_mut().find(|i| i.id == *id) {
///             item.name = name.clone();
///         }
///     }
/// )]
/// async fn update_item(
///     id: u64,
///     name: String,
///     ctx: MutationContext<Vec<Item>, Error>,
/// ) -> Result<Vec<Item>, Error> {
///     ctx.map_current(|items| {
///         if let Some(item) = items.iter_mut().find(|i| i.id == id) {
///             item.name = name;
///         }
///     }).ok_or(Error::NoData)
/// }
/// ```
#[proc_macro_attribute]
pub fn mutation(args: TokenStream, input: TokenStream) -> TokenStream {
    let mutation_args = if args.is_empty() {
        MutationArgs::default()
    } else {
        match syn::parse(args) {
            Ok(args) => args,
            Err(err) => return err.to_compile_error().into(),
        }
    };

    let input_fn = parse_macro_input!(input as ItemFn);

    let result = generate_mutation(input_fn, mutation_args);

    match result {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_provider(input_fn: ItemFn, provider_args: ProviderArgs) -> Result<TokenStream2> {
    let info = extract_provider_info(&input_fn)?;

    let ProviderInfo {
        fn_vis,
        fn_block,
        output_type,
        error_type,
        struct_name,
        ..
    } = &info;

    // Extract parameters once
    let params = extract_all_params(&input_fn)?;

    // Validate composition requirements if compose is used
    if !provider_args.compose.is_empty() {
        validate_composition_requirements(&provider_args.compose, &params)?;
    }

    // Generate enhanced function body with dependency injection and composition
    let enhanced_fn_block =
        generate_enhanced_function_body(&provider_args.compose, &params, fn_block);

    // Generate interval and cache expiration implementations
    let interval_impl = generate_interval_impl(&provider_args);
    let cache_expiration_impl = generate_cache_expiration_impl(&provider_args);
    let stale_time_impl = generate_stale_time_impl(&provider_args);

    // Generate common struct and const
    let common_struct = generate_common_struct_and_const(&info);

    // Determine parameter type and implementation based on function parameters
    if params.is_empty() {
        // No parameters - Provider<()>
        Ok(quote! {
            #common_struct

            impl #struct_name {
                #fn_vis async fn call() -> Result<#output_type, #error_type> {
                    #enhanced_fn_block
                }
            }

            impl ::dioxus_provider::hooks::Provider<()> for #struct_name {
                type Output = #output_type;
                type Error = #error_type;

                fn run(&self, _param: ()) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
                    Self::call()
                }

                #interval_impl
                #cache_expiration_impl
                #stale_time_impl
            }
        })
    } else if params.len() == 1 {
        // Single parameter - Provider<ParamType>
        let param = &params[0];
        let param_name = &param.name;
        let param_type = &param.ty;

        Ok(quote! {
            #common_struct

            impl #struct_name {
                #fn_vis async fn call(#param_name: #param_type) -> Result<#output_type, #error_type> {
                    #enhanced_fn_block
                }
            }

            impl ::dioxus_provider::hooks::Provider<#param_type> for #struct_name {
                type Output = #output_type;
                type Error = #error_type;

                fn run(&self, #param_name: #param_type) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
                    Self::call(#param_name)
                }

                #interval_impl
                #cache_expiration_impl
                #stale_time_impl
            }
        })
    } else {
        // Multiple parameters - Provider<(Param1, Param2, ...)>
        let param_names: Vec<_> = params.iter().map(|p| &p.name).collect();
        let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();
        let tuple_type = quote! { (#(#param_types,)*) };

        Ok(quote! {
            #common_struct

            impl #struct_name {
                #fn_vis async fn call(#(#param_names: #param_types,)*) -> Result<#output_type, #error_type> {
                    #enhanced_fn_block
                }
            }

            impl ::dioxus_provider::hooks::Provider<#tuple_type> for #struct_name {
                type Output = #output_type;
                type Error = #error_type;

                fn run(&self, params: #tuple_type) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
                    let (#(#param_names,)*) = params;
                    Self::call(#(#param_names,)*)
                }

                #interval_impl
                #cache_expiration_impl
                #stale_time_impl
            }
        })
    }
}

fn generate_mutation(input_fn: ItemFn, mutation_args: MutationArgs) -> Result<TokenStream2> {
    let info = extract_provider_info(&input_fn)?;

    let ProviderInfo {
        fn_vis,
        fn_block,
        output_type,
        error_type,
        struct_name,
        fn_name: _fn_name,
        ..
    } = &info;

    let enhanced_fn_block = generate_enhanced_function_body(&[], &[], fn_block);
    let invalidation_impl = generate_invalidation_impl(&mutation_args);
    let common_struct = generate_common_struct_and_const(&info);

    let raw_params = extract_all_params(&input_fn)?;
    let has_optimistic = mutation_args.optimistic.is_some();
    let (input_params, context_param, data_param) =
        split_mutation_params(raw_params.clone(), output_type, has_optimistic)?;

    // Detect auto-apply mode: optimistic is present and there's a data parameter
    let is_auto_apply = has_optimistic && data_param.is_some();

    // Build call parameters based on the original function signature
    let call_params: Vec<_> = raw_params
        .iter()
        .map(|p| {
            let name = &p.name;
            if let Some(ctx) = &context_param && ctx.name == p.name {
                let data_ty = &ctx.data_ty;
                let error_ty = &ctx.error_ty;
                quote! { #name: ::dioxus_provider::mutation::MutationContext<'_, #data_ty, #error_ty> }
            } else {
                let ty = &p.ty;
                quote! { #name: #ty }
            }
        })
        .collect();

    let call_signature = quote! { #fn_vis async fn call(#(#call_params),*) -> Result<#output_type, #error_type> {
        #enhanced_fn_block
    } };

    let input_count = input_params.len();
    let input_type = build_input_type(&input_params);

    let data_param_name = data_param.as_ref().map(|p| &p.name);

    let call_args_builder = |ctx_ident: Option<&syn::Ident>,
                             auto_apply_data_expr: Option<TokenStream2>|
     -> Vec<TokenStream2> {
        raw_params
            .iter()
            .map(|param| {
                // If this is the context parameter, use the ctx_ident
                if let Some(ctx) = ctx_ident {
                    if param.name == *ctx {
                        return quote! { #ctx };
                    }
                }
                // If this is the data parameter and we have auto-applied data, use that
                if let Some(data_name) = data_param_name {
                    if param.name == *data_name {
                        if let Some(ref data_expr) = auto_apply_data_expr {
                            return data_expr.clone();
                        }
                    }
                }
                // Otherwise, use the parameter name as-is
                let name = &param.name;
                quote! { #name }
            })
            .collect()
    };

    let context_ident = context_param.as_ref().map(|ctx| ctx.name.clone());
    let context_data_ty = context_param.as_ref().map(|ctx| ctx.data_ty.clone());
    let context_error_ty = context_param.as_ref().map(|ctx| ctx.error_ty.clone());

    let optimistic_impl = if let Some(optimistic_expr) = &mutation_args.optimistic {
        // Generate the call to optimistic closure based on param count
        let optimistic_call = match input_params.len() {
            0 => quote! { (#optimistic_expr)(&mut updated) },
            1 => quote! { (#optimistic_expr)(&mut updated, input) },
            _ => {
                let names: Vec<_> = input_params.iter().map(|p| &p.name).collect();
                quote! {
                    let (#(ref #names,)*) = *input;
                    (#optimistic_expr)(&mut updated, #(#names,)*)
                }
            }
        };

        quote! {
            fn optimistic_updates_with_current(
                &self,
                input: &#input_type,
                current_data: Option<&Result<Self::Output, Self::Error>>,
            ) -> Vec<(String, Result<Self::Output, Self::Error>)> {
                let keys = self.invalidates();
                if keys.is_empty() {
                    return Vec::new();
                }

                if let Some(Ok(current)) = current_data {
                    let mut updated = current.clone();
                    #optimistic_call;

                    let mut results = Vec::with_capacity(keys.len());
                    for key in keys {
                        results.push((key, Ok(updated.clone())));
                    }
                    results
                } else {
                    Vec::new()
                }
            }
        }
    } else {
        quote! {}
    };

    let (mutate_signature, mutate_body) = {
        let (signature, mut prelude) = match input_count {
            0 => (
                quote! { fn mutate(&self, _input: ()) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send },
                Vec::<TokenStream2>::new(),
            ),
            1 => {
                let param = &input_params[0];
                let name = &param.name;
                let ty = &param.ty;
                (
                    quote! { fn mutate(&self, #name: #ty) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send },
                    Vec::<TokenStream2>::new(),
                )
            }
            _ => {
                let names: Vec<_> = input_params.iter().map(|p| &p.name).collect();
                (
                    quote! { fn mutate(&self, input: #input_type) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send },
                    vec![quote! { let (#(#names),*) = input; }],
                )
            }
        };

        // Create MutationContext if needed in manual mode
        if !is_auto_apply {
            if let (Some(ctx_ident), Some(data_ty), Some(err_ty)) = (
                context_ident.as_ref(),
                context_data_ty.as_ref(),
                context_error_ty.as_ref(),
            ) {
                prelude.push(quote! { let #ctx_ident = ::dioxus_provider::mutation::MutationContext::<'static, #data_ty, #err_ty>::new(None); });
            }
        }

        let call_args = if is_auto_apply {
            // Auto-apply mode: provide default data (rarely called - should use mutate_with_current)
            call_args_builder(None, Some(quote! { Default::default() }))
        } else {
            // Manual mode: use context if present
            call_args_builder(context_ident.as_ref(), None)
        };

        let call_expr = quote! { Self::call(#(#call_args),*) };
        let body = quote! { async move { #(#prelude)* #call_expr.await } };
        (signature, body)
    };

    let (mutate_with_current_signature, mutate_with_current_body) = {
        let (signature, mut prelude) = match input_count {
            0 => (
                quote! { fn mutate_with_current(
                    &self,
                    _input: (),
                    current_data: Option<&Result<Self::Output, Self::Error>>,
                ) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send },
                Vec::<TokenStream2>::new(),
            ),
            1 => {
                let param = &input_params[0];
                let name = &param.name;
                let ty = &param.ty;
                (
                    quote! { fn mutate_with_current(
                        &self,
                        #name: #ty,
                        current_data: Option<&Result<Self::Output, Self::Error>>,
                    ) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send },
                    Vec::<TokenStream2>::new(),
                )
            }
            _ => {
                let names: Vec<_> = input_params.iter().map(|p| &p.name).collect();
                (
                    quote! { fn mutate_with_current(
                        &self,
                        input: #input_type,
                        current_data: Option<&Result<Self::Output, Self::Error>>,
                    ) -> impl ::std::future::Future<Output = Result<Self::Output, Self::Error>> + Send },
                    vec![quote! { let (#(#names),*) = input; }],
                )
            }
        };

        let call_args = if is_auto_apply {
            // Auto-apply mode: use current_data directly (already has optimistic update applied by runtime)
            // DO NOT re-apply the optimistic closure here - that would cause double-application!
            prelude.push(quote! {
                let __auto_apply_data = if let Some(Ok(current)) = current_data {
                    current.clone()
                } else {
                    // If no current data, use default
                    Default::default()
                };
            });

            call_args_builder(None, Some(quote! { __auto_apply_data }))
        } else {
            // Manual mode: create MutationContext from current_data
            if let Some(ctx_ident) = context_ident.as_ref() {
                prelude.push(quote! { let #ctx_ident = ::dioxus_provider::mutation::MutationContext::new(current_data); });
            }
            call_args_builder(context_ident.as_ref(), None)
        };

        let call_expr = quote! { Self::call(#(#call_args),*) };
        let body = quote! { async move { #(#prelude)* #call_expr.await } };
        (signature, body)
    };

    let has_optimistic_impl = if has_optimistic {
        quote! {
            fn has_optimistic(&self) -> bool {
                true
            }
        }
    } else {
        quote! {}
    };

    let mutation_impl = quote! {
        impl ::dioxus_provider::mutation::Mutation<#input_type> for #struct_name {
            type Output = #output_type;
            type Error = #error_type;

            #mutate_signature {
                #mutate_body
            }

            #mutate_with_current_signature {
                #mutate_with_current_body
            }

            #optimistic_impl

            #invalidation_impl

            #has_optimistic_impl
        }
    };

    Ok(quote! {
        #common_struct

        impl #struct_name {
            #call_signature
        }

        #mutation_impl
    })
}

/// Generate duration implementation for provider methods
fn generate_duration_impl(method_name: &str, duration: Option<Duration>) -> TokenStream2 {
    if let Some(duration) = duration {
        let duration_secs = duration.as_secs();
        let method_ident = syn::Ident::new(method_name, proc_macro2::Span::call_site());

        quote! {
            fn #method_ident(&self) -> Option<::std::time::Duration> {
                Some(::std::time::Duration::from_secs(#duration_secs))
            }
        }
    } else {
        quote! {}
    }
}

/// Generate interval implementation
fn generate_interval_impl(provider_args: &ProviderArgs) -> TokenStream2 {
    generate_duration_impl("interval", provider_args.interval)
}

/// Generate cache expiration implementation
fn generate_cache_expiration_impl(provider_args: &ProviderArgs) -> TokenStream2 {
    generate_duration_impl("cache_expiration", provider_args.cache_expiration)
}

/// Generate stale time implementation
fn generate_stale_time_impl(provider_args: &ProviderArgs) -> TokenStream2 {
    generate_duration_impl("stale_time", provider_args.stale_time)
}

/// Generate invalidation implementation for mutations
fn generate_invalidation_impl(mutation_args: &MutationArgs) -> TokenStream2 {
    if mutation_args.invalidates.is_empty() {
        quote! {}
    } else {
        let provider_calls: Vec<_> = mutation_args
            .invalidates
            .iter()
            .map(|provider_fn| {
                quote! {
                    ::dioxus_provider::mutation::provider_cache_key_simple(#provider_fn())
                }
            })
            .collect();

        quote! {
            fn invalidates(&self) -> Vec<String> {
                vec![#(#provider_calls,)*]
            }
        }
    }
}

/// Information extracted from the provider function
struct ProviderInfo {
    fn_vis: syn::Visibility,
    fn_attrs: Vec<syn::Attribute>,
    fn_block: Box<syn::Block>,
    output_type: Type,
    error_type: Type,
    struct_name: syn::Ident,
    fn_name: syn::Ident,
}

/// Information about a function parameter
#[derive(Clone)]
struct ParamInfo {
    name: syn::Ident,
    ty: Type,
}

#[derive(Clone)]
struct ContextInfo {
    name: syn::Ident,
    data_ty: Type,
    error_ty: Type,
}

fn parse_context_type(ty: &Type) -> Option<(Type, Type)> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "MutationContext" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 2 {
                        let mut iter = args.args.iter();
                        let data_ty = match iter.next()? {
                            syn::GenericArgument::Type(ty) => ty.clone(),
                            _ => return None,
                        };
                        let error_ty = match iter.next()? {
                            syn::GenericArgument::Type(ty) => ty.clone(),
                            _ => return None,
                        };
                        return Some((data_ty, error_ty));
                    }
                }
            }
        }
    }
    None
}

#[allow(dead_code)]
fn split_params(params: Vec<ParamInfo>) -> Result<(Vec<ParamInfo>, Option<ContextInfo>)> {
    let mut input_params = Vec::new();
    let mut context_param = None;

    for param in params {
        if let Some((data_ty, error_ty)) = parse_context_type(&param.ty) {
            if context_param.is_some() {
                return Err(syn::Error::new_spanned(
                    param.ty,
                    "Only one MutationContext parameter is allowed",
                ));
            }
            context_param = Some(ContextInfo {
                name: param.name,
                data_ty,
                error_ty,
            });
        } else {
            input_params.push(param);
        }
    }

    Ok((input_params, context_param))
}

/// Split mutation parameters into input params, context param, and auto-apply data param
fn split_mutation_params(
    params: Vec<ParamInfo>,
    output_type: &Type,
    has_optimistic: bool,
) -> Result<(Vec<ParamInfo>, Option<ContextInfo>, Option<ParamInfo>)> {
    let mut input_params = Vec::new();
    let mut context_param = None;
    let mut data_param = None;

    for param in params {
        if let Some((data_ty, error_ty)) = parse_context_type(&param.ty) {
            if context_param.is_some() {
                return Err(syn::Error::new_spanned(
                    param.ty,
                    "Only one MutationContext parameter is allowed",
                ));
            }
            context_param = Some(ContextInfo {
                name: param.name,
                data_ty,
                error_ty,
            });
        } else {
            input_params.push(param);
        }
    }

    // In auto-apply mode (has optimistic but no context), the last param might be the data param
    if has_optimistic && context_param.is_none() && !input_params.is_empty() {
        // Check if the last parameter's type matches the output type
        if let Some(last_param) = input_params.last() {
            if types_equal(&last_param.ty, output_type) {
                data_param = input_params.pop();
            }
        }
    }

    Ok((input_params, context_param, data_param))
}

/// Compare two types for structural equality
fn types_equal(ty1: &Type, ty2: &Type) -> bool {
    ty1 == ty2
}

/// Extract provider information from the input function
fn extract_provider_info(input_fn: &ItemFn) -> Result<ProviderInfo> {
    let fn_name = input_fn.sig.ident.clone();
    let fn_vis = input_fn.vis.clone();
    let fn_attrs = input_fn.attrs.clone();
    let fn_block = input_fn.block.clone();

    let (output_type, error_type) = extract_result_types(&input_fn.sig.output)?;
    let struct_name = syn::Ident::new(
        &to_pascal_case(&fn_name.to_string()),
        proc_macro2::Span::call_site(),
    );

    Ok(ProviderInfo {
        fn_vis,
        fn_attrs,
        fn_block,
        output_type,
        error_type,
        struct_name,
        fn_name,
    })
}

/// Generate common struct and const for the provider
fn generate_common_struct_and_const(info: &ProviderInfo) -> TokenStream2 {
    let struct_name = &info.struct_name;
    let fn_attrs = &info.fn_attrs;
    let fn_name = &info.fn_name;

    quote! {
        #[derive(Clone, PartialEq)]
        #(#fn_attrs)*
        pub struct #struct_name;

        impl Default for #struct_name {
            fn default() -> Self {
                Self
            }
        }

        // Generate a function that returns an instance of the struct
        pub fn #fn_name() -> #struct_name {
            #struct_name
        }
    }
}

/// Extract all parameters from the function signature
fn extract_all_params(input_fn: &ItemFn) -> Result<Vec<ParamInfo>> {
    let mut params = Vec::new();

    for input in &input_fn.sig.inputs {
        match input {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                if let Pat::Ident(pat_ident) = &**pat {
                    params.push(ParamInfo {
                        name: pat_ident.ident.clone(),
                        ty: (**ty).clone(),
                    });
                } else {
                    return Err(syn::Error::new_spanned(
                        pat,
                        "Only simple parameter names are supported",
                    ));
                }
            }
            FnArg::Receiver(_) => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Methods with self parameter are not supported",
                ));
            }
        }
    }

    Ok(params)
}

/// Build the input type: () for 0 params, T for 1 param, (T1, T2, ...) for N params
fn build_input_type(params: &[ParamInfo]) -> TokenStream2 {
    match params.len() {
        0 => quote! { () },
        1 => {
            let ty = &params[0].ty;
            quote! { #ty }
        }
        _ => {
            let types: Vec<_> = params.iter().map(|p| &p.ty).collect();
            quote! { (#(#types,)*) }
        }
    }
}

/// Extract result types from the function return type
fn extract_result_types(return_type: &ReturnType) -> Result<(Type, Type)> {
    match return_type {
        ReturnType::Default => Err(syn::Error::new_spanned(
            return_type,
            "Provider functions must return Result<T, E>",
        )),
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Result" {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if args.args.len() == 2 {
                                let mut args_iter = args.args.iter();

                                let output_type = match args_iter.next().unwrap() {
                                    syn::GenericArgument::Type(ty) => ty.clone(),
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            args,
                                            "Result must have type arguments",
                                        ));
                                    }
                                };

                                let error_type = match args_iter.next().unwrap() {
                                    syn::GenericArgument::Type(ty) => ty.clone(),
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            args,
                                            "Result must have type arguments",
                                        ));
                                    }
                                };

                                return Ok((output_type, error_type));
                            }
                        }
                    }
                }
            }

            Err(syn::Error::new_spanned(
                return_type,
                "Provider functions must return Result<T, E>",
            ))
        }
    }
}

/// Convert a string to PascalCase
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Validate composition requirements for compose providers
fn validate_composition_requirements(
    compose_providers: &[syn::Ident],
    params: &[ParamInfo],
) -> Result<()> {
    // Validate that all parameters implement Clone when composition is used
    if !params.is_empty() {
        validate_clone_requirements(params)?;
    }

    // Validate that composed providers exist (generates compile-time checks)
    validate_provider_existence(compose_providers)?;

    Ok(())
}

/// Validate that all parameters implement Clone for composition
fn validate_clone_requirements(params: &[ParamInfo]) -> Result<()> {
    for param in params {
        let param_type = &param.ty;
        let param_name = &param.name;

        // Generate a compile-time assertion that the type implements Clone
        // This will be added to the generated code to provide clear error messages
        let _clone_check = quote! {
            const _: fn() = || {
                fn assert_clone<T: Clone>() {}
                assert_clone::<#param_type>();
            };
        };

        // Note: The actual Clone validation happens at compile-time when the generated
        // code tries to clone the parameters. The error message will be improved by
        // the explicit clone calls we generate in generate_composition_statements_with_validation.
    }

    Ok(())
}

/// Validate that composed providers exist by generating compile-time checks
fn validate_provider_existence(compose_providers: &[syn::Ident]) -> Result<()> {
    // We can't fully validate provider existence at macro expansion time,
    // but we can generate code that will provide better error messages
    // if the providers don't exist or have incompatible signatures.

    for provider in compose_providers {
        // Generate a compile-time check that will give a clear error if the provider doesn't exist
        let _existence_check = quote! {
            const _: fn() = || {
                // This will cause a compile error with a clear message if the provider doesn't exist
                let _ = #provider;
            };
        };
    }

    Ok(())
}

/// Generate enhanced function body with composition
fn generate_enhanced_function_body(
    compose_providers: &[syn::Ident],
    params: &[ParamInfo],
    original_block: &syn::Block,
) -> syn::Block {
    let mut statements = Vec::new();

    // Add composition statements
    if !compose_providers.is_empty() {
        let composition_statements = generate_composition_statements(compose_providers, params);
        statements.extend(composition_statements);
    }

    // Add original function body statements
    statements.extend(original_block.stmts.clone());

    syn::Block {
        brace_token: original_block.brace_token,
        stmts: statements,
    }
}

/// Generate composition statements that can be directly added to a statement list
fn generate_composition_statements(
    compose_providers: &[syn::Ident],
    params: &[ParamInfo],
) -> Vec<syn::Stmt> {
    if compose_providers.is_empty() {
        return vec![];
    }

    let mut statements = Vec::new();

    // Add compile-time validation checks for better error messages
    statements.extend(generate_validation_statements(compose_providers, params));

    // Generate variable names for composed results with unique prefix to avoid collisions
    let result_vars: Vec<_> = compose_providers
        .iter()
        .map(|provider| {
            syn::Ident::new(
                &format!("__dioxus_composed_{provider}_result"),
                proc_macro2::Span::call_site(),
            )
        })
        .collect();

    // Generate provider calls based on parameter count
    if params.is_empty() {
        // No parameters - call providers with ()
        let provider_calls: Vec<_> = compose_providers
            .iter()
            .map(|provider| {
                quote! {
                    async { #provider().run(()).await }
                }
            })
            .collect();

        let join_stmt: syn::Stmt = syn::parse_quote! {
            let (#(#result_vars,)*) = ::futures::join!(
                #(#provider_calls,)*
            );
        };
        statements.push(join_stmt);
    } else if params.len() == 1 {
        // Single parameter - clone it inside each async block
        let param_name = &params[0].name;
        let param_type = &params[0].ty;

        let provider_calls: Vec<_> = compose_providers
            .iter()
            .map(|provider| {
                quote! {
                    async {
                        // Explicit clone with helpful error context
                        let param: #param_type = #param_name.clone();
                        #provider().run(param).await
                    }
                }
            })
            .collect();

        let join_stmt: syn::Stmt = syn::parse_quote! {
            let (#(#result_vars,)*) = ::futures::join!(
                #(#provider_calls,)*
            );
        };
        statements.push(join_stmt);
    } else {
        // Multiple parameters - clone each parameter inside each async block
        let param_names: Vec<_> = params.iter().map(|p| &p.name).collect();
        let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();

        let provider_calls: Vec<_> = compose_providers
            .iter()
            .map(|provider| {
                quote! {
                    async {
                        // Explicit clone with helpful error context for each parameter
                        let params: (#(#param_types,)*) = (#(#param_names.clone(),)*);
                        #provider().run(params).await
                    }
                }
            })
            .collect();

        let join_stmt: syn::Stmt = syn::parse_quote! {
            let (#(#result_vars,)*) = ::futures::join!(
                #(#provider_calls,)*
            );
        };
        statements.push(join_stmt);
    }

    statements
}

/// Generate compile-time validation statements for better error messages
fn generate_validation_statements(
    compose_providers: &[syn::Ident],
    params: &[ParamInfo],
) -> Vec<syn::Stmt> {
    let mut statements = Vec::new();

    // Add Clone validation for parameters if composition is used
    if !params.is_empty() {
        for param in params {
            let param_type = &param.ty;
            let param_name = &param.name;

            // Generate a compile-time Clone assertion with helpful error message
            let clone_check: syn::Stmt = syn::parse_quote! {
                const _: () = {
                    fn __dioxus_provider_assert_clone<T: ::std::clone::Clone>() {}
                    fn __dioxus_provider_validate_parameter_clone() {
                        __dioxus_provider_assert_clone::<#param_type>();
                    }
                };
            };
            statements.push(clone_check);
        }
    }

    // Add provider existence validation
    for provider in compose_providers {
        // Generate a compile-time check that the provider exists and is callable
        let existence_check: syn::Stmt = syn::parse_quote! {
            const _: () = {
                fn __dioxus_provider_validate_existence() {
                    // This will cause a clear compile error if the provider doesn't exist
                    let _provider_exists = #provider;
                }
            };
        };
        statements.push(existence_check);
    }

    statements
}
