use std::option;
use pyth_sdk::{PriceFeed,Price};
use futures::stream::StreamExt; // For stream processing methods
use std::pin::Pin;
use pyth_hermes_client::PythClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new Pyth client
    let client = PythClient::new(
        "https://hermes.pyth.network".parse().unwrap()
    );
    
    // SUI/USD price feed ID
    let sui_price_id:String = String::from("0x23d7315113f5b1d3ba7a83604c44b94d79f4fd69af77f804fc7f920a6dc65744");
    let v: Vec<String> = vec![sui_price_id.clone()];
    // Get the latest price update
    let  price_updates= client.stream_price_updates(v,None,Some(true),Some(false),Some(true)).await?;
    //process the streamtype price_updates
    
    // Process the stream
    price_updates
        .peek() async {
            match update_result {
                Ok(price_update) => {
                    println!("Received price update:");
                    
                    // Access parsed data if available
                    if let Some(parsed) = &price_update.parsed {
                        for price_feed in parsed {
                            let price = price_feed.price.price as f64 * 10.0_f64.powi(price_feed.price.expo);
                            println!("Symbol: {}, Price: ${:.4}", price_feed.id, price);
                            println!("Confidence: {}, Timestamp: {}", 
                                     price_feed.price.conf, 
                                     price_feed.price.publish_time);
                        }
                    }
                },
                Err(err) => {
                    eprintln!("Error receiving price update: {:?}", err);
                }
            }
        })
        .await;
    
    Ok(())
}
