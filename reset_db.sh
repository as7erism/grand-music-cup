#!/usr/bin/env bash

sqlx database drop -y
sqlx database create
sqlx migrate run
