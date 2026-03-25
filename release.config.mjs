/**
 * @type {import('semantic-release').GlobalConfig}
 */
export default {
  branches: [
    "main",
    {name: "development", prerelease: "beta"}
  ],
  plugins: [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    [
      "@semantic-release/github",
      {
        "assets": [
          { "path": "target/release/libhermes-linux-x86_64.so", "label": "Linux x86_64" },
          { "path": "target/release/libhermes-linux-aarch64.so", "label": "Linux ARM64" },
          { "path": "target/release/libhermes-macos-aarch64.dylib", "label": "MacOS ARM64" },
          { "path": "target/release/libhermes-macos-x86_64.dylib", "label": "MacOS x86_64" },
          { "path": "target/release/libhermes-windows-x86_64.dll", "label": "Windows x86_64" },
        ]
      }
    ]
  ]
}
