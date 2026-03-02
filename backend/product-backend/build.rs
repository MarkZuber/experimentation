fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        // Add serde derives to all proto message types used in HTTP responses
        .type_attribute("items.Item", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("items.ItemResponse", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("items.ListItemsResponse", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("cache.KeyValuePair", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("cache.ListKeysResponse", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["../proto/items.proto", "../proto/cache.proto"],
            &["../proto"],
        )?;
    Ok(())
}
