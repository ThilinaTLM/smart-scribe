#!/bin/bash

kill -SIGUSR1 $(cat /tmp/smart-scribe.pid)
