#[cfg(test)]
mod test {
    use async_trait::async_trait;
    use bb8::Pool;
    use sidekiq::{
        BalanceStrategy, Processor, ProcessorConfig, QueueConfig, RedisConnectionManager,
        RedisPool, Result, WorkFetcher, Worker,
    };
    use std::sync::{Arc, Mutex};

    #[async_trait]
    trait FlushAll {
        async fn flushall(&self);
    }

    #[async_trait]
    impl FlushAll for RedisPool {
        async fn flushall(&self) {
            let mut conn = self.get().await.unwrap();
            let _: String = redis::cmd("FLUSHALL")
                .query_async(conn.unnamespaced_borrow_mut())
                .await
                .unwrap();
        }
    }

    async fn new_base_processor(queue: String) -> (Processor, RedisPool) {
        // Redis
        let manager = RedisConnectionManager::new("redis://127.0.0.1/").unwrap();
        let redis = Pool::builder().build(manager).await.unwrap();
        redis.flushall().await;

        // Sidekiq server
        let p = Processor::new(redis.clone(), vec![queue]).with_config(
            ProcessorConfig::default()
                .num_workers(1)
                .balance_strategy(BalanceStrategy::RoundRobin)
                .queue_config(
                    "dedicated queue 1".to_string(),
                    QueueConfig::default().num_workers(10),
                )
                .queue_config(
                    "dedicated queue 2".to_string(),
                    QueueConfig::default().num_workers(100),
                ),
        );

        (p, redis)
    }

    #[tokio::test]
    async fn can_process_an_async_job() {
        #[derive(Clone)]
        struct TestWorker {
            did_process: Arc<Mutex<bool>>,
        }

        #[async_trait]
        impl Worker for TestWorker {
            type Args = ();
            async fn perform(&self, _args: ()) -> Result<()> {
                let mut this = self.did_process.lock().unwrap();
                *this = true;

                Ok(())
            }
        }

        let worker = TestWorker {
            did_process: Arc::new(Mutex::new(false)),
        };
        let queue = "random123".to_string();
        let (mut p, redis) = new_base_processor(queue.clone()).await;

        p.register(worker.clone());

        TestWorker::opts()
            .queue(queue)
            .perform_async(&redis, ())
            .await
            .unwrap();

        assert_eq!(p.process_one_tick_once().await.unwrap(), WorkFetcher::Done);
        assert!(*worker.did_process.lock().unwrap());
    }
}
