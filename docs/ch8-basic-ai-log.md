# AI Collaboration Notes for Chapter 8 Basic Experiment

## 1. How AI Was Used

AI was used as a collaborative engineering assistant, not as a direct code generator.

The workflow was:

1. read `codex.md`, `exercise.md`, Chapter 8 source files, user tests, and checker cases
2. infer the exact required behavior from test programs
3. narrow down the minimal implementation path
4. diagnose failures based on actual logs and test output
5. verify the final implementation in Docker

## 2. Key Problems AI Helped Identify

### 2.1 Mutex deadlock is not the same as semaphore deadlock

AI helped separate the two models:

- mutex can be handled as a wait-for graph
- semaphore needs a safety-style resource analysis to avoid false positives

### 2.2 Resource transfer semantics matter

AI helped identify that:

- `mutex.unlock()` may transfer ownership directly to the waking thread
- `semaphore.up()` may also transfer one released resource to a blocked waiter

If the deadlock metadata is not updated accordingly, detection becomes inconsistent with real kernel state.

### 2.3 Passing output is not enough without full verification

AI was also used to keep the verification discipline strict:

- compile first
- run base tests
- run exercise tests
- confirm all expected outputs and checker counts

## 3. Learning Outcome

The main learning gain was not only completing the Chapter 8 code, but also understanding how to:

- model thread and resource state in the kernel
- distinguish blocking from deadlock
- use AI to accelerate localization and validation without giving up semantic judgment
