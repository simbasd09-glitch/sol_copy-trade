use crate::trader::Event;
use crate::config::BotConfig;
use tokio::time::{sleep, Duration};

pub struct HeliusSubscriber {
    cfg: BotConfig,
}

impl HeliusSubscriber {
    pub fn new(cfg: BotConfig) -> Self {
        Self { cfg }
    }

    // For now this is a mock that simulates incoming events.
    // Replace this method to connect to Helius gRPC and forward events.
    pub async fn subscribe<F>(self, mut handler: F)
    where
        F: FnMut(Event) + Send + 'static,
    {
        let cfg = self.cfg;
        tokio::spawn(async move {
            loop {
                // Simulate a dev create event and buy event occasionally
                let ev_create = Event::ProgramCreate {
                    mint: "ExampleMint11111111111111111111111111111111".to_string(),
                    dev: "DevPubkey11111111111111111111111111111111".to_string(),
                };
                handler(ev_create);

                sleep(Duration::from_secs(5)).await;

                let ev_buy = Event::Buy {
                    trader: "DevPubkey11111111111111111111111111111111".to_string(),
                    mint: "ExampleMint11111111111111111111111111111111".to_string(),
                    sol: 0.05,
                    token_amount: 1000.0,
                };
                handler(ev_buy);

                sleep(Duration::from_secs(30)).await;
            }
        });
    }
}
