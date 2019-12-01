use futures::prelude::*;
use std::collections::HashMap;
use upnp::{ssdp::URN, Device, Uri};

#[async_std::main]
async fn main() -> Result<(), upnp::Error> {
    let url = Uri::from_static("http://192.168.2.49:1400/xml/device_description.xml");
    let service_urn = URN::service("schemas-upnp-org", "ZoneGroupTopology", 1);

    let device = Device::from_url(url).await?;
    let service = device.find_service(&service_urn).unwrap();

    let (sid, mut stream) = service.subscribe(device.url(), 10).await?;

    while let Some(state_vars) = stream.next().await {
        handle(state_vars?);
        service.renew_subscription(device.url(), &sid, 10).await?;
    }

    Ok(())
}

fn handle(state_vars: HashMap<String, String>) {
    println!("Change {{");
    for (key, value) in state_vars {
        if value.len() > 256 {
            println!("  {}: ...", key);
        } else {
            println!("  {}: {}", key, value);
        }
    }
    println!("}}");
}
