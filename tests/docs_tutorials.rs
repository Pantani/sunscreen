#[test]
fn first_nft_tutorial_uses_supported_quickstart_arguments() {
    let tutorial = std::fs::read_to_string("docs/site/src/learn/your-first-nft.md")
        .expect("read first NFT tutorial");

    assert!(
        tutorial.contains("sunscreen quickstart nft --name my-first-nft --cluster devnet"),
        "tutorial should use the clap-supported --name quickstart form"
    );
    assert!(
        !tutorial.contains("sunscreen quickstart nft my-first-nft"),
        "quickstart accepts the project name through --name, not a second positional argument"
    );
}
