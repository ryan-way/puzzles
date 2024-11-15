use sea_orm::{Database, DatabaseConnection, DbErr};

pub async fn get_connection() -> Result<DatabaseConnection, DbErr> {
    Database::connect(dotenv!("DATABASE_URL")).await
}
