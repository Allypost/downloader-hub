mod processor;
pub mod task;

use app_logger::trace;
use deadqueue::unlimited::Queue;
use once_cell::sync::Lazy;
pub use processor::TaskQueueProcessor;
use task::Task;

static TASK_QUEUE: Lazy<Queue<Task>> = Lazy::new(Queue::new);

pub struct TaskQueue;
impl TaskQueue {
    pub fn push(task: Task) {
        trace!(?task, "Pushing task to queue");
        TASK_QUEUE.push(task);
    }
}