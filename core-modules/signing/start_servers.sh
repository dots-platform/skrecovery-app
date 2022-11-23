#!/usr/bin/env bash
node_count=`yq eval '.nodes | length' server_conf.yml`

cd ..
cd ..

for i in $(seq 1 $node_count)
do
    echo "Waking up node $i out of $node_count"
    ./platform/init_server --node_id node$i --config ./core-modules/signing/server_conf.yml &
    sleep 3
done

