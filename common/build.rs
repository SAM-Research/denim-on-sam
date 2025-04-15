fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .type_attribute("DeniableMessage", "#[derive(bon::Builder)]")
        .type_attribute("UserMessage", "#[derive(bon::Builder)]")
        .type_attribute("BlockRequest", "#[derive(bon::Builder)]")
        .type_attribute("KeyRequest", "#[derive(bon::Builder)]")
        .type_attribute("KeyResponse", "#[derive(bon::Builder)]")
        .type_attribute("KeyUpdate", "#[derive(bon::Builder)]")
        .type_attribute("SeedUpdate", "#[derive(bon::Builder)]")
        .type_attribute("Error", "#[derive(bon::Builder)]")
        .type_attribute("DummyPadding", "#[derive(bon::Builder)]")
        .type_attribute("DenimMessage", "#[derive(bon::Builder)]")
        .type_attribute("KeyBundle", "#[derive(bon::Builder)]")
        .include_file("_includes.rs")
        .compile_protos(&["proto/DenimMessage.proto"], &["proto"])?;

    Ok(())
}
