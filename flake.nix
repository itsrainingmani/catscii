{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
        flake-utils.follows = "flake-utils";
      };
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = true;
          };
          # this refers to the path ./rust-toolchain.toml
          rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          # tell crane to use our toolchain & use our private shipyard registry
          craneLibWithoutRegistry = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          shipyardToken = builtins.readFile ./secrets/shipyard-token;
          craneLib = craneLibWithoutRegistry.appendCrateRegistries [
            (craneLibWithoutRegistry.registryFromDownloadUrl {
              dl = "https://crates.shipyard.rs/api/v1/crates";
              indexUrl = "ssh://git@ssh.shipyard.rs/itsrainingmani/crate-index.git";
              fetchurlExtraArgs = {
                curlOptsList = [ "-H" "user-agent: shipyard ${shipyardToken}" ];
              };
            })
          ];
          src = craneLib.cleanCargoSource ./.;
          # things you only need at compile-time
          nativeBuildInputs = with pkgs; [ rustToolchain pkg-config sqlite ];
          # things you also need at run-time
          buildInputs = with pkgs; [ openssl ];
          # this will be used for both `cargoArtifacts` and `bin`
          commonArgs = {
            inherit src buildInputs nativeBuildInputs;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          # remember, `set1 // set2` does a shallow merge:
          bin = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
          dockerImage = pkgs.dockerTools.streamLayeredImage {
            name = "catscii";
            tag = "latest";
            contents = [ bin pkgs.cacert ];
            config = {
              Cmd = [ "${bin}/bin/catscii" ];
              Env = with pkgs; [ "GEOLITE2_COUNTRY_DB=${clash-geoip}/etc/clash/Country.mmdb" ];
            };
          };
        in
        with pkgs;
        {
          packages =
            {
              # so we can build `bin` specifically
              # but it's also the default
              inherit bin dockerImage;
              default = bin;
            };
          devShells.default = mkShell {
            #refer to an existing derivation
            inputsFrom = [ bin ];
            buildInputs = with pkgs; [ dive flyctl just ];
          };
        }
      );
}
