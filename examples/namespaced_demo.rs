use async_trait::async_trait;
use bb8::Pool;
use sidekiq::{Processor, RedisConnectionManager, Result, Worker};

#[derive(Clone)]
struct HelloWorker;

#[async_trait]
impl Worker for HelloWorker {
    type Args = ();

    async fn perform(&self, _args: Self::Args) -> Result<()> {
        println!("Hello, world!");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Redis
    let manager = RedisConnectionManager::new("redis://127.0.0.1/")?;
    let redis = Pool::builder()
        .max_size(100)
        .connection_customizer(sidekiq::with_custom_namespace("yolo_app".to_string()))
        .build(manager)
        .await?;

    tokio::spawn({
        let redis = redis.clone();

        async move {
            loop {
                HelloWorker::perform_async(&redis, ()).await.unwrap();

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });

    // Sidekiq server
    let mut p = Processor::new(redis.clone(), vec!["default".to_string()]);

    // Add known workers
    p.register(HelloWorker);

    // Start!
    p.run().await;
    Ok(())
}
