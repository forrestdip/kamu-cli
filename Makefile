.PHONY: bootstrap
bootstrap:
	cd lib/ \
	&& rm -f *.jar \
	&& wget "https://github.com/kamu-data/hive/releases/download/1.2.1.spark2.kamu.1/hive-jdbc-1.2.1.spark2.kamu.1-standalone.jar"