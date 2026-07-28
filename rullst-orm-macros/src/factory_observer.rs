use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let factory_name = quote::format_ident!("{}Factory", name);
    let observer_trait_name = quote::format_ident!("{}Observer", name);

    quote! {
        pub struct #factory_name {
            generator: Box<dyn Fn() -> #name + Send + Sync>,
            count: usize,
        }

        impl #factory_name {
            pub fn new(generator: impl Fn() -> #name + Send + Sync + 'static) -> Self {
                Self {
                    generator: Box::new(generator),
                    count: 1,
                }
            }

            pub fn count(mut self, count: usize) -> Self {
                self.count = count;
                self
            }

            pub async fn create(&self) -> Result<Vec<#name>, rullst_orm::Error> {
                let mut futures = vec![];
                for _ in 0..self.count {
                    let mut model = (self.generator)();
                    futures.push(async move {
                        model.save().await?;
                        Ok::<#name, rullst_orm::Error>(model)
                    });
                }
                let results = rullst_orm::_futures::future::try_join_all(futures).await?;
                Ok(results)
            }

            pub fn make(&self) -> Vec<#name> {
                let mut results = vec![];
                for _ in 0..self.count {
                    results.push((self.generator)());
                }
                results
            }
        }

        impl #name {
            pub fn factory<F: 'static + Send + Sync + Fn() -> #name>(generator: F) -> #factory_name {
                #factory_name {
                    generator: Box::new(generator),
                    count: 1,
                }
            }
        }

        #[rullst_orm::async_trait]
        pub trait #observer_trait_name {
            async fn saving(&self, model: &mut #name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn saved(&self, model: &#name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn updating(&self, model: &mut #name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn updated(&self, model: &#name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn creating(&self, model: &mut #name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn created(&self, model: &#name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn deleting(&self, model: &#name) -> Result<(), rullst_orm::Error> { Ok(()) }
            async fn deleted(&self, model: &#name) -> Result<(), rullst_orm::Error> { Ok(()) }
        }
    }
}
