use async_trait::async_trait;
use bb8::Pool;
use serde::{Deserialize, Serialize};
use sidekiq::{
    ChainIter, Job, RedisConnectionManager, Result, ServerMiddleware, Worker, WorkerRef,
};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Clone)]
struct HelloWorker;

#[async_trait]
impl Worker for HelloWorker {
    type Args = ();

    async fn perform(&self, _args: Self::Args) -> Result<()> {
        // I don't use any args. I do my own work.
        Ok(())
    }
}

#[derive(Clone)]
struct PaymentReportWorker {}

impl PaymentReportWorker {
    async fn send_report(&self, user_guid: String) -> Result<()> {
        // TODO: Some actual work goes here...
        info!({"user_guid" = user_guid, "class_name" = Self::class_name()}, "Sending payment report to user");

        Ok(())
    }
}

#[derive(Deserialize, Debug, Serialize)]
struct PaymentReportArgs {
    user_guid: String,
}

#[async_trait]
impl Worker for PaymentReportWorker {
    type Args = PaymentReportArgs;

    fn opts() -> sidekiq::WorkerOpts<Self> {
        sidekiq::WorkerOpts::new().queue("yolo")
    }

    async fn perform(&self, args: Self::Args) -> Result<()> {
        self.send_report(args.user_guid).await
    }
}

struct FilterExpiredUsersMiddleware {}

#[derive(Deserialize)]
struct FiltereExpiredUsersArgs {
    user_guid: String,
}

impl FiltereExpiredUsersArgs {
    fn is_expired(&self) -> bool {
        self.user_guid == "USR-123-EXPIRED"
    }
}

#[async_trait]
impl ServerMiddleware for FilterExpiredUsersMiddleware {
    async fn call(
        &self,
        chain: ChainIter,
        job: &Job,
        worker: Arc<WorkerRef>,
        redis: Pool<RedisConnectionManager>,
    ) -> Result<()> {
        let args: std::result::Result<(FiltereExpiredUsersArgs,), serde_json::Error> =
            serde_json::from_value(job.args.clone());

        // If we can safely deserialize then attempt to filter based on user guid.
        if let Ok((filter,)) = args {
            if filter.is_expired() {
                error!({
                        "class" = &job.class,
                        "jid" = &job.jid,
                        "user_guid" = filter.user_guid
                    },
                    "Detected an expired user, skipping this job"
                );
                return Ok(());
            }
        }

        chain.next(job, worker, redis).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Redis
    let manager = RedisConnectionManager::new("redis://127.0.0.1/")?;
    let redis = Pool::builder().build(manager).await?;

    let mut n = 0;
    let mut last = 0;
    let mut then = std::time::Instant::now();

    loop {
        PaymentReportWorker::perform_async(
            &redis,
            PaymentReportArgs {
                user_guid: "USR-123".into(),
            },
        )
        .await
        .unwrap();

        //tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        n += 1;

        if n % 100000 == 0 {
            let now = std::time::Instant::now();
            let delta = n - last;
            last = n;
            let delta_time = now - then;
            if delta_time.as_secs() == 0 {
                continue;
            }
            then = now;
            let rate = delta / delta_time.as_secs();
            println!("Iterations since last: {delta} at a rate of: {rate} iter/sec");
        }
    }

    //    // Enqueue a job with the worker! There are many ways to do this.
    //    PaymentReportWorker::perform_async(
    //        &mut redis,
    //        PaymentReportArgs {
    //            user_guid: "USR-123".into(),
    //        },
    //    )
    //    .await?;
    //
    //    PaymentReportWorker::perform_in(
    //        &mut redis,
    //        std::time::Duration::from_secs(10),
    //        PaymentReportArgs {
    //            user_guid: "USR-123".into(),
    //        },
    //    )
    //    .await?;
    //
    //    PaymentReportWorker::opts()
    //        .queue("brolo")
    //        .perform_async(
    //            &mut redis,
    //            PaymentReportArgs {
    //                user_guid: "USR-123-EXPIRED".into(),
    //            },
    //        )
    //        .await?;
    //
    //    sidekiq::perform_async(
    //        &mut redis,
    //        "PaymentReportWorker".into(),
    //        "yolo".into(),
    //        PaymentReportArgs {
    //            user_guid: "USR-123".to_string(),
    //        },
    //    )
    //    .await?;
    //
    //    // Enqueue a job
    //    sidekiq::perform_async(
    //        &mut redis,
    //        "PaymentReportWorker".into(),
    //        "yolo".into(),
    //        PaymentReportArgs {
    //            user_guid: "USR-123".to_string(),
    //        },
    //    )
    //    .await?;
    //
    //    // Enqueue a job with options
    //    sidekiq::opts()
    //        .queue("yolo".to_string())
    //        .perform_async(
    //            &mut redis,
    //            "PaymentReportWorker".into(),
    //            PaymentReportArgs {
    //                user_guid: "USR-123".to_string(),
    //            },
    //        )
    //        .await?;

    //    // Sidekiq server
    //    let mut p = Processor::new(
    //        redis.clone(),
    //        logger.clone(),
    //        //vec!["yolo".to_string(), "brolo".to_string()],
    //        vec![],
    //    );
    //
    //    //    // Add known workers
    //    //    p.register(HelloWorker);
    //    //    p.register(PaymentReportWorker::new(logger.clone()));
    //    //
    //    // Custom Middlewares
    //    p.using(FilterExpiredUsersMiddleware::new(logger.clone()))
    //        .await;
    //
    //    // Reset cron jobs
    //    periodic::destroy_all(redis.clone()).await?;
    //
    //    // Cron jobs
    //    periodic::builder("0 * * * * *")?
    //        .name("Payment report processing for a random user")
    //        .queue("yolo")
    //        //.args(PaymentReportArgs {
    //        //    user_guid: "USR-123-PERIODIC".to_string(),
    //        //})?
    //        .args(json!({ "user_guid": "USR-123-PERIODIC" }))?
    //        .register(&mut p, PaymentReportWorker::new(logger.clone()))
    //        .await?;
    //
    //    p.run().await;
}
