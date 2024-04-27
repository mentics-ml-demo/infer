#!/usr/bin/env bash

# echo -e "cqlsh\n$(cat ddl.cql)"

echo -e "SELECT * FROM ml_demo.inference;" | docker exec -i scylla cqlsh