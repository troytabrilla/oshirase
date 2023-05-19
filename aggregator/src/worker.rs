use crate::Aggregator;

use redis::Commands;
use time::OffsetDateTime;

const DEFAULT_RETRY_TIMEOUT: u64 = 10;

pub struct Worker<'a> {
    aggregator: &'a mut Aggregator<'a>,
}

impl<'a> Worker<'a> {
    pub fn new(aggregator: &'a mut Aggregator<'a>) -> Worker<'a> {
        Worker { aggregator }
    }

    fn get_retry_timeout(&self) -> usize {
        self.aggregator.config.worker.retry_timeout
    }

    fn get_retry_timeout_duration(&self) -> std::time::Duration {
        let retry_timeout = self
            .get_retry_timeout()
            .try_into()
            .unwrap_or(DEFAULT_RETRY_TIMEOUT);

        std::time::Duration::from_secs(retry_timeout)
    }

    pub async fn run(&mut self) {
        let retry_timeout = self.aggregator.config.worker.retry_timeout;

        let client = &self.aggregator.db.redis.client.clone();
        loop {
            let connection = client.get_connection_with_timeout(self.get_retry_timeout_duration());
            match connection {
                Ok(mut connection) => {
                    let job = connection.brpoplpush::<&str, &str, Option<String>>(
                        "aggregator:worker:jobs",
                        "aggregator:worker:failed",
                        retry_timeout,
                    );
                    match job {
                        Ok(msg) => {
                            if let Some(msg) = msg {
                                if &msg == "run:all" {
                                    println!(
                                        "Running aggregator for {}: {}.",
                                        msg,
                                        OffsetDateTime::now_utc()
                                    );
                                    let start = std::time::Instant::now();
                                    match self.aggregator.run(None).await {
                                        Ok(_) => println!(
                                            "Finished running aggregator: {:?}.",
                                            start.elapsed()
                                        ),
                                        Err(err) => eprintln!("Could not run aggregator: {}", err),
                                    };
                                }
                            }

                            if let Err(err) = connection.del::<&str, ()>("aggregator:worker:failed")
                            {
                                eprintln!("Could not clear failed queue: {}", err);
                            }
                        }
                        Err(err) => eprintln!("Could not get job: {}", err),
                    }
                }
                Err(err) => eprintln!("Could not establish connection: {}", err),
            }
        }
    }
}
