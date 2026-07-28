use std::fs;
use rullst_orm::Orm;

#[derive(Debug, Clone, Orm)]
pub struct FuzzUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Orm)]
#[orm(searchable)]
pub struct ScoutUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::main]
async fn main() {
    let _ = rullst_orm::Orm::init("sqlite::memory:").await;
    let path = r"C:\Users\venelouis\Desktop\ARTEFATOS\oom-95b38d36f8f594e0cfc2ba01e29dadc9fbe0c624";
    let data = fs::read(path).expect("Failed to read OOM artifact");
    
    if let Ok(s) = std::str::from_utf8(&data) {
        println!("Testing Scout...");
        let _ = ScoutUser::search(s);
        println!("Scout finished");
        
        println!("Testing Builder...");
        let builder = FuzzUser::query()
            .where_like("name", s)
            .where_eq("email", s)
            .order_by_desc(s)
            .limit(10);
            
        let _sql = builder.to_sql();
        println!("Builder finished");
    }
}
