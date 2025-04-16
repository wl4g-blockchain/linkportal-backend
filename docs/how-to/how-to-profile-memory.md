# Profile memory usage of linkportalbackend

This crate provides an easy approach to dump memory profiling info.

## Prerequisites

### jemalloc

```bash
# for macOS
brew install jemalloc

# for Ubuntu
sudo apt install libjemalloc-dev
```

### [flamegraph](https://github.com/brendangregg/FlameGraph)

```bash
curl https://raw.githubusercontent.com/brendangregg/FlameGraph/master/flamegraph.pl > ./flamegraph.pl 
```

### Build linkportalbackend with `profiling-mem-prof` feature

```bash
cargo build --features=profiling-mem-prof
```

## Profiling

- Start linkportalbackend instance with environment variables: (Set the stack trace sampling interval of jemalloc to 2^28 bytes i.e. about 256 MB)

```bash
MALLOC_CONF=prof:true,lg_prof_interval:28 ./target/debug/linkportalbackend
```

Dump memory profiling data through HTTP API:

```bash
curl localhost:9001/debug/prof/mem > linkportalbackend.hprof
```

You can periodically dump profiling data and compare them to find the delta memory usage.

## Analyze profiling data with flamegraph

To create flamegraph according to dumped profiling data:

```bash
jeprof --svg <path_to_linkportalbackend_binary> --base=<baseline_prof> <profile_data> > output.svg
```
