use async_trait::async_trait;

#[async_trait]
pub trait Repository<T>: Send + Sync {
    type Id: Send + Sync;
    type Error: Send + Sync;

    async fn find_by_id(&self, id: Self::Id) -> Result<Option<T>, Self::Error>;
    async fn find_all(&self) -> Result<Vec<T>, Self::Error>;
    async fn save(&self, entity: &T) -> Result<(), Self::Error>;
    async fn delete(&self, id: Self::Id) -> Result<(), Self::Error>;
}

pub struct GenericRepository<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> Default for GenericRepository<T> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> GenericRepository<T> {
    pub fn new() -> Self {
        Self::default()
    }
}
