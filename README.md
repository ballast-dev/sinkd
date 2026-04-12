# ![image](resources/imgs/sinkd-black-txt.png)

## _Don't get out of `sinkd`_ ⚓︎

_**Everything and the kitchen sink**_  

## Deployable Cloud

_True Privacy_  
I believe your files should stay with you **always**.  

- **No** third party eyes
- **No** privacy policies
- **No** tradeoffs

Given the pleathora of cloud providers and the frequent data breaches of large companies 
I created `sinkd` to give the power back to the user. With an old machine, or a spare 
Raspberry Pi, you can create your own __local__ cloud! The purpose and goal of this program
is to get out of the users way while reducing the overhead of keeping up environments 
across many computers. 

1. Cross-Platform (*nix, Windows(WSL)), built for ARM and x86
1. Wraps `rsync` into a daemon keeping track of file events
1. Physical access to your files
1. Granular permissions per user  
1. Uses DDS (Data Distribution Service) for peer-to-peer communication - no broker required

### Installation

**macOS:** use Homebrew with the formula in this repo:

```bash
brew install --head ./pkg/homebrew/sinkd.rb
```

See comments in [`pkg/homebrew/sinkd.rb`](pkg/homebrew/sinkd.rb) for a versioned install from a release tarball.

**Linux / Windows:** prebuilt binaries and packages (`.deb`, `.pkg.tar.zst`, Windows zip) attach to [GitHub Releases](https://github.com/ballast-dev/sinkd/releases) when you tag a version.

**Arch Linux (aarch64):** CI does not build Arch packages on ARM64 runners (no suitable multi-arch `archlinux` image in practice). Use the `linux-arm64` binary from the release, build [`pkg/arch/create.sh`](pkg/arch/create.sh) locally, or install another distro’s artifact.

### Roadmap

- use `btrfs` on a virtual mount for ease of snapshotting
- add encryption to files watched by sinkd
- access from outside the LAN firewall
- use `git-lfs` to enable data restore (manual tagging)
