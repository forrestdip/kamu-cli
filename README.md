# Kamu CLI

# MVP
- V define dataset manifest formats and schemas
- V setup verbose logging
- V implement root poll command with passing a manifest in
- V ftp support
- V integrate transform
- V Multi-file support
- V Get rid of `spark-warehouse` folder
- V Replace cache json with yaml
- V Use docker to run spark
- V Put GeoSpark into image, not dependencies
- V Create PySpark + Livy image
- V Create Jupyter + SparkMagic + Kamu image
- V Create `notebook` command to launch Jupyter
- V Permission fix-up for `notebook` command
- V Rethink repository structure and how datasets are linked
- V Implement `pull` command to refresh dataset
- V Implement `list` command showing datasets and their metadata
- V DOT graph output
- V Referential integrity
- V `delete` command
- V `purge` command
- V Root dataset creation wizard (generate based on schema)
- V Control output verbosity
- V Add `sql --server` command that starts Livy with JDBC endpoint exposed
- V Add `sql` interactive shell
- V Add support for one-off `sql` commands
- Livy sessions should be pre-configured with GeoSpark types
- Implement `describe` command showing dataset metadata
- Implement file `import` and `export`
- Simplify historical vocab
- Distribution `arch`
- Distribution `ubuntu`
- Distribution `mac`
- Implement `status` command showing which data is out-of-date
- Implement recursive mode for `pull` command
- Handle dependencies during `purge`
- Write version metainfo for later upgrades
- Force `pull` to update uncacheable datasets
- Figure out "unable to infer schema" when loading folder of parquets
- Prettify `help`
- Allow adding manifest from remote URL
- Create "stable" repository of known good datasources

# Post-MVP
- V Suppress HDFS warning on startup
- Add bash completions
- Add `spark-shell` command (scala and python)
- Upgrade to latest Spark
- Fix notebook warnings
- Avoid permission issues by propagating UID/GID into containers
- Cleanup poll data
- Lock repository where necessary to prevent concurrent alteration
- Avoid hard-setting the container names
- Use local timezone in all logs
- Allow using custom-built docker images

# Known issues
- `Transaction isolation level` warning on sql shell startup
- `NullPointerException` when creating temporary view in sql shell
- `NoSuchElementException` on every SQL syntax error instead of root cause
