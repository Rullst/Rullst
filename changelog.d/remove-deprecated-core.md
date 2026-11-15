### Removed

- **Breaking:** `rullst_core::Queue::purge_completed_jobs` (`rullst::queue::Queue`),
  deprecated in 12.0.0, was removed. Despite its name it removed failed jobs:
  call `Queue::purge_failed_jobs`, which does the same, or
  `purge_completed_history` to remove completed jobs where the driver keeps
  them. The `QueueDriver::purge_completed_jobs` hook that custom drivers
  implement is unchanged. `cargo rullst upgrade` reports
  `.purge_completed_jobs()` calls in files that name `Queue`
  (`V13-QUEUE-PURGE-COMPLETED-REMOVED`, a review finding, because a
  `QueueDriver` still has that method).
