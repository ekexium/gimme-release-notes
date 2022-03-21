A tool to pick every release note from the TiKV (and propably others) repo in a certani range of commits.

## Usage
For example, 

to find all release notes for TiKV from realease 5.3 to release 5.4, 
run `GITHUB_TOKEN=<your_github_token> cargo run -- -r tikv/tikv --range release-5.3...release-5.4 -o out.txt`

to find all release notes for TiDB from release 5.4 to master
`GITHUB_TOKEN=<your_github_token> cargo run -- -r pingcap/tidb --range release-5.4...master -o out.txt`

It gives the output like:
```
e298e935116d30be34f72b13296452807f3e20c1
[#10833](https://github.com/tikv/tikv/pull/10833)
release-note
Added disk protection mechanism to try to avoid panic caused by exhaustion of disk space.
Support to use `Drop/Truncate Table` to release space when the disk is full.
Expansion should be early rather than late.


42c7aad5fb93d0ee6a69179e314ddc8ffc474364
[#11205](https://github.com/tikv/tikv/pull/11205)
release-note
status_server: skip profiling sample in glibc, pthread, libgcc to avoid possible deadlock 
status_server: upgrade pprof-rs to fix memory leak


0390c764e4457b9c94c132d971316439c54a1524
[#10948](https://github.com/tikv/tikv/pull/10948)
release-note
Fix rocksdb panic after ingesting two files in once call.
```

**NOTE**: it relies on the `release-note` label and the template that tikv/tikv uses to find the release note.