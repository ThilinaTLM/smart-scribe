#!/bin/bash

kill -SIGUSR2 $(cat /tmp/smart-scribe.pid)
