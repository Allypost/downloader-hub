#[tokio::main]
#[allow(unreachable_code)]
async fn main() {
    #[cfg(all(feature = "server", feature = "cli", not(debug_assertions)))]
    {
        return compile_error!("Can't use both `cli` and `server` features at the same time");
    }

    #[cfg(feature = "server")]
    {
        return app_server::run().await;
    }
}
