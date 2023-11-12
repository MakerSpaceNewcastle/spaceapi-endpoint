{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";

    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    naersk,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        toolchain = fenix.packages.${system}.toolchainOf {
          channel = "1.72";
          date = "2023-09-19";
          sha256 = "dxE7lmCFWlq0nl/wKcmYvpP9zqQbBitAQgZ1zx9Ooik=";
        };

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain.rust;
          rustc = toolchain.rust;
        };

        cargo = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        version = cargo.package.version;

        nativeBuildInputs = with pkgs; [cmake pkg-config];
        buildInputs = with pkgs; [openssl];

        lintingRustFlags = "-D unused-crate-dependencies";
      in {
        devShell = pkgs.mkShell {
          nativeBuildInputs = nativeBuildInputs;
          buildInputs = buildInputs;

          packages = with pkgs; [
            toolchain.toolchain

            # Code formatting tools
            alejandra
            treefmt

            # Container image management tool
            skopeo
          ];

          RUSTFLAGS = lintingRustFlags;
        };

        packages = rec {
          default = naersk'.buildPackage {
            name = "makerspace-spaceapi";
            version = version;

            src = ./.;

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
          };

          container-image = pkgs.dockerTools.buildImage {
            name = "makerspace-spaceapi";
            tag = "latest";
            created = "now";

            copyToRoot = pkgs.buildEnv {
              name = "image-root";
              paths = with pkgs; [bashInteractive coreutils];
              pathsToLink = ["/bin"];
            };

            config = {
              Entrypoint = ["${pkgs.tini}/bin/tini" "--" "${default}/bin/makerspace-spaceapi"];
              ExposedPorts = {
                "8080/tcp" = {};
                "9090/tcp" = {};
              };
              Env = [
                "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                "API_ADDRESS=0.0.0.0:8080"
                "OBSERVABILITY_ADDRESS=0.0.0.0:9090"
              ];
            };
          };

          clippy = naersk'.buildPackage {
            src = ./.;
            mode = "clippy";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
          };

          test = naersk'.buildPackage {
            src = ./.;
            mode = "test";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;

            # Ensure detailed test output appears in nix build log
            cargoTestOptions = x: x ++ ["1>&2"];
          };
        };
      }
    );
}
