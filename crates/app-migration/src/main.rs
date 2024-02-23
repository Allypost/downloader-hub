use sea_orm_migration::prelude::*;

pub const DEBUG: bool = true;

#[async_std::main]
async fn main() {
    cli::run_cli(app_migration::Migrator).await;
}
