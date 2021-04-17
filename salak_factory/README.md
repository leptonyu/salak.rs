#salak_factory


### example toy_log
```bash
cargo install --example toy_log --features='enable_log' --path .
```

### toy_logger vs env_logger
```bash
time toy_log -P count=10000000 > /dev/null           
# Record 10000000 logs in 3905ms, 390ns/log, 10242994/s, 2560748/s/thread
# toy_log -P count=10000000 > /dev/null  3.60s user 0.15s system 380% cpu 0.985 total


time toy_log -P count=10000000 -P env_log=true > /dev/null      
# Record 10000000 logs in 70219ms, 7021ns/log, 569642/s, 142410/s/thread
# toy_log -P count=10000000 -P env_log=true > /dev/null  16.42s user 35.60s system 287% cpu 18.064 total
```