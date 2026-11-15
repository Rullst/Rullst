### Changed

- **Breaking:** `Server`'s access log no longer writes each `[HTTP]` line to
  stdout from the request. Every request wrote and flushed its line while
  holding the stdout lock, so a slow reader (a pipe, a container log driver)
  held up the requests that finished meanwhile and the Tokio workers queued on
  the lock. A background thread now writes the queued lines in batches, a few
  milliseconds after their responses. The queue holds 8192 lines and 2 MiB;
  when stdout cannot keep up and it is full, further lines are dropped instead
  of slowing the server, and the next batch writes `[HTTP] access log: N lines
  dropped because stdout did not keep up`. A graceful shutdown waits up to one
  second for the queued lines; a killed process loses them.
