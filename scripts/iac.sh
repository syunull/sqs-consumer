#!/usr/bin/env bash

docker exec -t localstack-cli aws sqs create-queue --queue-name sqs-consumer
