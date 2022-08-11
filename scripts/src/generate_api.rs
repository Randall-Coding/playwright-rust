use case::CaseExt;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use scripts::{api::*, utils};

fn main() {
    let api: Api = serde_json::from_reader(std::io::stdin()).unwrap();
    let t = to_tokens(&api);
    println!("{}\n// vim: foldnestmax=0 ft=rust", t);
}

fn to_tokens(api: &Api) -> TokenStream {
    let mut tokens = TokenStream::default();
    tokens.append_all(api.0.iter().map(body));
    tokens
}

fn body(x: &Interface) -> TokenStream {
    let name = format_ident!("{}", utils::loud_to_camel(&x.name));
    let mod_name = format_ident!("{}", utils::loud_to_camel(&x.name).to_snake());
    let extends = x.extends.as_deref().map(|e| {
        let e = format!("Extends {}", e);
        quote! { #[doc=#e] }
    });
    // TODO: doc_comment
    let methods = x
        .members
        .iter()
        .filter(|m| matches!(m.kind, Kind::Property | Kind::Method))
        .map(Method);
    // let sub = collect_types(x);
    // let properties = self.properties();
    quote! {
        mod #mod_name {
            #extends
            impl #name {
                #(#methods)*
            }
        }
    }
}

#[derive(Debug)]
struct Method<'a>(&'a Member);

impl<'a> ToTokens for Method<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Method(Member {
            kind: _,
            name,
            alias,
            experimental,
            since,
            overload_index,
            required,
            is_async,
            args,
            ty,
            deprecated,
            spec
        }) = self;
        let rety = Use(ty);
        let arg_fields = args.iter().filter(|a| a.required).map(arg_field);
        let is_builder = {
            // two or more optional values
            let mut xs = args.iter().filter(|a| !a.required).chain(
                args.iter()
                    .filter(|a| a.name == "options" && !a.ty.properties.is_empty())
                    .flat_map(|a| a.ty.properties.iter())
            );
            xs.next().and(xs.next()).is_some()
        };
        let fn_name = format_ident!(
            "{}{}",
            utils::loud_to_snake(&name.replace("#", "")),
            if is_builder { "_builder" } else { "" }
        );
        let mark_async = (!is_builder && *is_async)
            .then(|| quote!(async))
            .unwrap_or_default();
        // TODO: doc_comment
        let doc_unnecessary = (!required)
            .then(|| quote!(#[doc="unnecessary"]))
            .unwrap_or_default();
        let doc_experimental = experimental
            .then(|| quote!(#[doc="experimental"]))
            .unwrap_or_default();
        let mark_deprecated = deprecated
            .then(|| quote!(#[deprecated]))
            .unwrap_or_default();
        tokens.extend(quote! {
            #doc_unnecessary
            #doc_experimental
            #mark_deprecated
            #mark_async fn #fn_name(#(#arg_fields),*) -> #rety {
                todo!()
            }
        })
    }
}

fn arg_field(a: &Arg) -> TokenStream {
    let Arg {
        name,
        kind: _,
        alias,
        ty,
        since,
        overload_index,
        spec,
        required,
        deprecated,
        is_async
    } = a;
    assert_eq!(*is_async, false);
    let field_name = format_ident!("{}", utils::loud_to_snake(name));
    let use_ty = Use(ty);
    let doc_deprecated = deprecated
        .then(|| quote!(#[doc="deprecated"]))
        .unwrap_or_default();
    // TODO: doc_comment
    // TODO: deprecated
    // TODO: required
    quote! {
        #field_name: #use_ty
    }
}

fn collect_types(x: &Interface) -> TokenStream {
    fn add<'a>(dest: &mut Vec<(String, &'a Type)>, prefix: String, t: &'a Type) {
        let Type {
            name,
            expression: _,
            properties,
            templates,
            union
        }: &Type = t;
        dest.push((prefix.clone(), t));
        for arg in properties {
            add(
                dest,
                format!("{}{}", &prefix, &arg.name.to_camel()),
                &arg.ty
            );
        }
        for ty in templates {
            add(dest, prefix.clone(), ty);
        }
        for ty in union {
            add(dest, prefix.clone(), ty);
        }
    }
    let mut ret = TokenStream::default();
    for member in &x.members {
        let mut types = Vec::new();
        for arg in &member.args {
            let name = format!("{}{}", &member.name.to_camel(), &arg.name.to_camel());
            add(&mut types, name, &arg.ty);
        }
        add(&mut types, member.name.to_camel(), &member.ty);
        let mod_name = format_ident!("{}", utils::loud_to_snake(&member.name.replace("#", "")));
        let mut types = types
            .into_iter()
            .map(|(prefix, ty)| Declare { prefix, ty })
            .peekable();
        if types.peek().is_some() {
            ret.extend(quote! {
                pub mod #mod_name {
                    #(#types)*
                }
            });
        }
    }
    ret
}

#[derive(Debug)]
struct Declare<'a> {
    prefix: String,
    ty: &'a Type
}

struct Use<'a>(&'a Type);

impl<'a> ToTokens for Declare<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Declare { prefix, ty } = self;
        if ty.union.is_empty() {
            if ty.properties.is_empty() && ty.templates.is_empty() {
                return;
            }
            let name = format_ident!("{}", prefix.replace("#", ""));
            match (ty.properties.is_empty(), ty.templates.is_empty()) {
                (true, true) => return,
                (false, false) => {
                    assert_eq!(ty.name, "Object");
                }
                (false, true) => {
                    assert_eq!(ty.name, "Object");
                    let properties = ty.properties.iter().map(|p| {
                        let deprecated = p
                            .deprecated
                            .then(|| quote!(#[deprecated]))
                            .unwrap_or_default();
                        let name = format_ident!("{}", utils::loud_to_snake(&p.name));
                        let orig = &p.name;
                        // TODO: doc_comment
                        let use_ty = {
                            let a = Use(&p.ty);
                            if p.required {
                                quote!(#a)
                            } else {
                                quote!(Option<#a>)
                            }
                        };
                        quote! {
                            #deprecated
                            #[serde(rename = #orig)]
                            #name: #use_ty
                        }
                    });
                    tokens.extend(quote! {
                        #[derive(Debug, Serialize, Deserialize)]
                        struct #name {
                            #(#properties),*
                        }
                    });
                }
                (true, false) => {}
            }
        } else {
            // let name = format_ident!("{}", &ty.name);
            let name = format_ident!("{}", prefix.replace("#", ""));
            let variants = ty.union.iter().map(|t| {
                if t.name.contains("\"") {
                    let name = t.name.replace("\"", "");
                    let label = format_ident!("{}", utils::kebab_to_camel(&name));
                    quote! {
                        #[serde(rename = #name)]
                        #label
                    }
                } else {
                    let name = &t.name;
                    let label = format_ident!("{}", utils::kebab_to_camel(&name));
                    let use_ty = Use(t);
                    quote! {
                            #[serde(rename = #name)]
                            #label(#use_ty)
                    }
                }
            });
            tokens.extend(quote! {
                enum #name {
                    #(#variants),*
                }
            });
        }
    }
}

impl<'a> ToTokens for Use<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! {
            ()
        });
    }
}

// impl Interface {
//     fn body(&self) -> TokenStream {
//        let name = self.name();
//        let extends = self.extends.as_deref().map(|e| {
//            let e = format!("Extends {}", e);
//            quote! { #[doc=#e] }
//        });
//        let comment = &self.comment;
//        let methods = self.methods();
//        let properties = self.properties();
//        let events = self.events();
//        let declares = self.collect_declares();
//        quote! {
//            #[doc = #comment]
//            #extends
//            impl #name {
//                #properties
//                #methods
//            }
//            #declares
//            #events
//        }
//    }

//     fn name(&self) -> Ident { format_ident!("{}", fix_loud_camel(&self.name)) }

//     fn extends(&self) -> Option<TokenStream> {
//        self.extends.as_ref().map(|e| {
//            let e = format_ident!("{}", e);
//            quote! { :#e }
//        })
//     }

//     fn properties(&self) -> TokenStream {
//        let ps = self
//            .members
//            .iter()
//            .filter(|m| m.kind == Kind::Property)
//            .map(|m| Property {
//                name: &self.name,
//                body: m
//            });
//        quote! {
//            #(#ps)*
//        }
//    }

//     fn methods(&self) -> TokenStream {
//        let ms = self
//            .members
//            .iter()
//            .filter(|m| m.kind == Kind::Method)
//            .map(|m| Method {
//                name: &self.name,
//                body: m
//            });
//        quote! {
//            #(#ms)*
//        }
//    }

//     fn events(&self) -> TokenStream {
//        let es = self
//            .members
//            .iter()
//            .filter(|x| x.kind == Kind::Event)
//            .map(|x| Event {
//                name: &self.name,
//                body: x
//            });
//        if es.clone().next().is_none() {
//            return quote! {};
//        }
//        let labels = es.clone().map(|e| {
//            let label = format_ident!("{}", e.body.name.to_camel());
//            let comment = &e.body.comment;
//            quote! {
//                #[doc=#comment]
//                #label
//            }
//        });
//        let bodies = es.map(|e| {
//            let label = format_ident!("{}", e.body.name.to_camel());
//            let t = &e.body.ty;
//            let comment = &e.body.comment;
//            quote! {
//                #[doc=#comment]
//                #label(#t)
//            }
//        });
//        let et = format_ident!("{}EventType", self.name);
//        let e = format_ident!("{}Event", self.name);
//        quote! {
//            enum #et {
//                #(#labels),*
//            }
//            enum #e {
//                #(#bodies),*
//            }
//        }
//    }

//     fn collect_declares(&self) -> TokenStream {
//        let mut res: TokenStream = quote! {};
//        for member in &self.members {
//            res.extend(member.ty.declare(&member.name));
//            for arg in member.args.iter().filter(|a| a.name != "options") {
//                res.extend(arg.ty.declare(&arg.name));
//            }
//        }
//        res
//    }
//}

// struct Event<'a, 'b> {
//    name: &'a str,
//    body: &'b Member
//}
// struct Method<'a, 'b> {
//    name: &'a str,
//    body: &'b Member
//}
// struct Property<'a, 'b> {
//    name: &'a str,
//    body: &'b Member
//}

// impl ToTokens for Method<'_, '_> {
//    fn to_tokens(&self, tokens: &mut TokenStream) {
//        let name = self.name();
//        let comment = &self.body.comment;
//        let ty = &self.body.ty;
//        let err = if self.body.is_async {
//            quote! {Arc<Error>}
//        } else {
//            quote! {Error}
//        };
//        let required = self
//            .body
//            .args
//            .iter()
//            .filter(|a| a.required)
//            .map(|a| a.with_colon());
//        let opts = self
//            .body
//            .args
//            .iter()
//            .filter(|a| !a.required && a.name != "options")
//            .map(|a| a.with_colon_option());
//        let options = self
//            .body
//            .args
//            .iter()
//            .filter(|a| !a.required && a.name == "options")
//            .map(|a| {
//                let xs = a.ty.properties.iter().map(|a| a.with_colon_option());
//                quote! { #[doc = "options"] #(#xs),* }
//            });
//        let all = required.chain(opts).chain(options);
//        tokens.extend(quote! {
//            #[doc = #comment]
//            fn #name(&self, #(#all),*) -> Result<#ty, #err> { todo!() }
//        });
//    }
//}

// impl Method<'_, '_> {
//    fn name(&self) -> Ident { format_ident!("{}", escape(&self.body.name.to_snake())) }
//}

// impl ToTokens for Property<'_, '_> {
//    fn to_tokens(&self, tokens: &mut TokenStream) {
//        let name = ident_snake(&self.body.name);
//        let comment = &self.body.comment;
//        let ty = &self.body.ty;
//        tokens.extend(quote! {
//            #[doc = #comment]
//            pub fn #name(&self) {}
//        });
//    }
//}

// fn ident_snake(name: &str) -> Ident { format_ident!("{}", escape(&name.to_snake())) }

// impl ToTokens for Type {
//    fn to_tokens(&self, tokens: &mut TokenStream) {
//        if !self.templates.is_empty() {
//            tokens.extend(match self.name.as_str() {
//                "Array" => self.array(),
//                "Object" | "Map" => self.map(),
//                "Func" => todo!("{:?}", self),
//                _ => unreachable!("{:?}", self)
//            });
//            return;
//        }
//        if !self.properties.is_empty() {
//            if self.name.is_empty() || self.name == "Object" {
//                tokens.extend(quote! { NotImplementedYet });
//            } else {
//                unreachable!();
//            }
//            return;
//        }
//        if !self.union.is_empty() {
//            let optional = self.union.iter().find(|u| u.name == "null");
//            if self.name.is_empty() {
//                let t = self.r#enum();
//                tokens.extend(optional.map(|_| quote! { Option<#t> }).unwrap_or(t));
//            } else {
//                let name = format_ident!("{}", self.name);
//                let name = quote! { #name };
//                tokens.extend(optional.map(|_| quote! { Option<#name> }).unwrap_or(name));
//            }
//            return;
//        }
//        match self.name.as_str() {
//            "" => {
//                unreachable!()
//            }
//            "Object" => {
//                tokens.extend(quote! { Object });
//                return;
//            }
//            "void" => {
//                tokens.extend(quote! { () });
//                return;
//            }
//            "string" => {
//                tokens.extend(quote! { String });
//                return;
//            }
//            "boolean" => {
//                tokens.extend(quote! { bool });
//                return;
//            }
//            "JSHandle" => {
//                tokens.extend(quote! { JsHandle });
//                return;
//            }
//            "int" => {
//                tokens.extend(quote! { i64 });
//                return;
//            }
//            "float" => {
//                tokens.extend(quote! { f64 });
//                return;
//            }
//            // any Any Serializable path
//            n => {
//                let name = if n == "System.Net.HttpStatusCode" {
//                    format_ident!("u16")
//                } else if n == r#""gone""# {
//                    format_ident!("Gone")
//                } else if n.starts_with('"') && n.ends_with('"') {
//                    // TODO
//                    format_ident!("{}", n[1..(n.len() - 1)])
//                } else {
//                    format_ident!("{}", n)
//                };
//                tokens.extend(quote! {
//                    #name
//                });
//                return;
//            }
//        }
//    }
//}

// impl Type {
//    fn function(&self) -> TokenStream { todo!() }

//    fn array(&self) -> TokenStream {
//        let t = self.templates.iter().next().unwrap();
//        quote! {
//            Vec<#t>
//        }
//    }

//    fn map(&self) -> TokenStream {
//        let fst = self.templates.iter().next().unwrap();
//        let snd = self.templates.iter().next().unwrap();
//        quote! {
//            Map<#fst, #snd>
//        }
//    }

//    fn r#enum(&self) -> TokenStream {
//        let mut entries = self.union.iter().filter(|u| u.name != "null");
//        let num = entries.clone().count();
//        match num {
//            0 => unreachable!(),
//            1 => {
//                let first = entries.next().unwrap();
//                quote! { #first }
//            }
//            _ => {
//                quote! {
//                    NotImplementedYet
//                }
//            }
//        }
//    }

//    // TODO: recursive
//    fn declare(&self, hint: &str) -> Option<TokenStream> {
//        if !self.properties.is_empty() && self.name != "function" {
//            let name = format_ident!("NotImplementedYet{}", hint);
//            let required = self
//                .properties
//                .iter()
//                .filter(|a| a.required)
//                .map(|a| a.with_colon());
//            let opts = self
//                .properties
//                .iter()
//                .filter(|a| !a.required)
//                .map(|a| a.with_colon_option());
//            let all = required.chain(opts);
//            let nested = self
//                .properties
//                .iter()
//                .map(|a| a.ty.declare(&a.name))
//                .fold(quote! {}, |mut a, b| {
//                    a.extend(b);
//                    a
//                });
//            return Some(quote! {
//                struct #name {
//                    #(#all),*
//                }
//                #nested
//            });
//        } else if !self.union.is_empty() {
//            let name = format_ident!("NotImplementedYet{}", hint);
//            let not_null = self.union.iter().filter(|u| u.name != "null");
//            if not_null.clone().count() <= 1 {
//                return None;
//            }
//            let nested = not_null
//                .clone()
//                .map(|t| t.declare(""))
//                .fold(quote! {}, |mut a, b| {
//                    a.extend(b);
//                    a
//                });
//            let xs = not_null.map(|t| {
//                quote! { NotImplementedYet(#t) }
//            });
//            return Some(quote! {
//                enum #name {
//                    #(#xs),*
//                }
//                #nested
//            });
//        }
//        None
//    }
//}

// impl Arg {
//    fn with_colon(&self) -> TokenStream {
//        let name = self.name();
//        let ty = &self.ty;
//        let comment = &self.comment;
//        quote! {
//            #[doc = #comment]
//            #name: #ty
//        }
//    }

//    fn with_colon_option(&self) -> TokenStream {
//        let name = self.name();
//        let ty = &self.ty;
//        let comment = &self.comment;
//        quote! {
//            #[doc = #comment]
//            #name: Option<#ty>
//        }
//    }

//    fn name(&self) -> Ident { format_ident!("{}", escape(&self.name.to_snake())) }
//}
