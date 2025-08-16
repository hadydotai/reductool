use inflector::Inflector;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, FnArg, ItemFn, Pat, PatIdent, Type, parse_macro_input};

fn path_last_ident_is(path: &syn::Path, name: &str) -> bool {
    path.segments
        .last()
        .map(|seg| seg.ident == name)
        .unwrap_or(false)
}

fn path_ends_with(path: &syn::Path, segments: &[&str]) -> bool {
    if segments.is_empty() {
        return false;
    }
    let pathlen = path.segments.len();
    if pathlen < segments.len() {
        return false;
    }
    path.segments
        .iter()
        .skip(pathlen - segments.len())
        .zip(segments.iter())
        .all(|(a, b)| a.ident == *b)
}

fn first_generic_arg<'a>(tp: &'a syn::TypePath) -> Option<&'a Type> {
    tp.path.segments.last().and_then(|seg| {
        if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
            ab.args.iter().find_map(|ga| {
                if let syn::GenericArgument::Type(t) = ga {
                    Some(t)
                } else {
                    None
                }
            })
        } else {
            None
        }
    })
}

fn primitive_json_type_name(path: &syn::Path) -> Option<&'static str> {
    let ident = path.segments.last()?.ident.to_string();
    Some(match ident.as_str() {
        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" | "isize"
        | "usize" => "integer",
        "f32" | "f64" => "number",
        "bool" => "boolean",
        "String" | "str" => "string",
        _ => return None,
    })
}

fn ty_to_schema(ty: &Type) -> serde_json::Value {
    match ty {
        // &T => T
        Type::Reference(r) => ty_to_schema(&r.elem),

        // [T; N] => array of T
        Type::Array(arr) => serde_json::json!({
            "type": "array",
            "items": ty_to_schema(&arr.elem),
        }),

        // (T1, T2, ...) -> fixed-length array with item schemas
        Type::Tuple(t) => {
            let items: Vec<serde_json::Value> = t.elems.iter().map(ty_to_schema).collect();
            serde_json::json!({
                "type": "array",
                "items": items,
                "minItems": items.len(),
                "maxItems": items.len(),
            })
        }

        // T paths: primitives, String, Vec<T>, Option<T>, serde_json::Value, etc.
        Type::Path(tp) => {
            if path_last_ident_is(&tp.path, "Vec") {
                if let Some(inner) = first_generic_arg(tp) {
                    return serde_json::json!({
                        "type": "array",
                        "items": ty_to_schema(inner),
                    });
                }
                return serde_json::json!({
                    "type": "array",
                    "items": { "type": "string" },
                });
            }

            if path_last_ident_is(&tp.path, "Option") {
                if let Some(inner) = first_generic_arg(tp) {
                    return ty_to_schema(inner);
                }
                return serde_json::Value::Object(serde_json::Map::new());
            }

            if let Some(json_ty) = primitive_json_type_name(&tp.path) {
                return serde_json::json!({"type": json_ty});
            }

            if path_ends_with(&tp.path, &["serde_json", "Value"])
                || path_last_ident_is(&tp.path, "Value")
            {
                return serde_json::Value::Object(serde_json::Map::new());
            }

            serde_json::json!({"type": "string"})
        }

        _ => serde_json::json!({"type": "string"}),
    }
}

fn collect_doc(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            let Ok(nv) = attr.meta.require_name_value() else {
                return None;
            };
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            else {
                return None;
            };
            Some(s.value())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[proc_macro_attribute]
pub fn aitool(_attr: TokenStream, code: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(code);
    let funcsig = &func.sig;
    let func_name = funcsig.ident.to_string();
    let doc = collect_doc(&func.attrs);

    let is_async = funcsig.asyncness.is_some();

    let mut fields = Vec::new();
    let mut field_names = Vec::new();
    let mut field_idents: Vec<syn::Ident> = Vec::new();
    let mut args = serde_json::Map::new();
    let mut required_args = Vec::new();
    let mut errors: Vec<syn::Error> = Vec::new();

    for input in &funcsig.inputs {
        match input {
            FnArg::Typed(pat_ty) => {
                //NOTE(@hadydotai): We'll only work with simple identifier patterns for now
                let param_ident = match &*pat_ty.pat {
                    Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                    _ => {
                        errors.push(syn::Error::new_spanned(
                            &pat_ty.pat,
                            "unsupported parameter pattern. expected a simple identifier like `arg: T`.\n\
                            Examples of unsupported patterns: `(_: T)`, `(a, b): (T, U)`, `S { x, y }: S`."
                        ));
                        continue;
                    }
                };

                let pat_ty_ty = &pat_ty.ty;
                let param_name = param_ident.to_string();
                fields.push(quote!(pub #param_ident: #pat_ty_ty));
                field_names.push(param_name.clone());
                field_idents.push(param_ident.clone());
                let schema = ty_to_schema(pat_ty_ty);
                args.insert(param_name.clone(), schema);
                let mut is_optional = false;
                if let Type::Path(tp) = &*pat_ty.ty {
                    if path_last_ident_is(&tp.path, "Option") {
                        is_optional = true;
                    }
                }
                if !is_optional {
                    required_args.push(param_name);
                }
            }
            FnArg::Receiver(recv) => {
                errors.push(syn::Error::new_spanned(
                    recv,
                    "#[aitool] must be placed on a free-standing function (no `self`).\
                    Move the function out of the `impl` block or remove the receiver.",
                ));
            }
        }
    }

    if !errors.is_empty() {
        let compile_errors = errors.into_iter().map(|err| err.to_compile_error());
        return quote! { #(#compile_errors)* }.into();
    }

    let args_struct_ident = syn::Ident::new(
        &format!("{}Args", func_name.to_table_case().to_pascal_case()),
        funcsig.ident.span(),
    );

    let fields_tokens = quote!(#(#fields),*);
    let required_array = serde_json::Value::Array(
        required_args
            .iter()
            .map(|arg| serde_json::Value::String(arg.clone()))
            .collect(),
    );

    let mut schema = serde_json::Map::new();
    schema.insert(
        "name".to_string(),
        serde_json::Value::String(func_name.clone()),
    );
    schema.insert(
        "description".to_string(),
        serde_json::Value::String(doc.clone()),
    );

    let mut parameters = serde_json::Map::new();
    parameters.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    parameters.insert("properties".to_string(), serde_json::Value::Object(args));
    parameters.insert("required".to_string(), required_array);

    schema.insert(
        "parameters".to_string(),
        serde_json::Value::Object(parameters),
    );
    let json_schema = serde_json::to_string(&schema).unwrap();
    let name_lit = syn::LitStr::new(&func_name, funcsig.ident.span());
    let desc_lit = syn::LitStr::new(&doc, funcsig.ident.span());
    let json_schema_lit = syn::LitStr::new(&json_schema, funcsig.ident.span());

    let func_wrapper_name = syn::Ident::new(
        &format!("__invoke_{}", func_name.clone()),
        funcsig.ident.span(),
    );
    let reg_name = syn::Ident::new(
        &format!("__REG_{}", func_name.clone().to_screaming_snake_case()),
        funcsig.ident.span(),
    );

    let ident = &funcsig.ident;
    let invoke_fn = if is_async {
        quote! {
            fn #func_wrapper_name(args: ::serde_json::Value) -> ::reductool::InvokeFuture {
                Box::pin(async move {
                    let parsed: #args_struct_ident = ::serde_json::from_value(args)?;
                    let out = #ident(#(parsed.#field_idents),*).await;
                    ::serde_json::to_value(out).map_err(Into::into)
                })
            }
        }
    } else {
        quote! {
            fn #func_wrapper_name(args: ::serde_json::Value) -> ::reductool::InvokeFuture {
                Box::pin(async move {
                    let parsed: #args_struct_ident = ::serde_json::from_value(args)?;
                    let out = #ident(#(parsed.#field_idents),*);
                    ::serde_json::to_value(out).map_err(Into::into)
                })
            }
        }
    };

    let expanded = quote! {
        #func

        #[derive(::serde::Deserialize)]
        struct #args_struct_ident {
            #fields_tokens
        }

        #invoke_fn

        #[::reductool::__linkme::distributed_slice(::reductool::ALL_TOOLS)]
        static #reg_name: ::reductool::ToolDefinition = ::reductool::ToolDefinition {
            name: #name_lit,
            description: #desc_lit,
            json_schema: #json_schema_lit,
            invoke: #func_wrapper_name,
        };
    };
    expanded.into()
}

