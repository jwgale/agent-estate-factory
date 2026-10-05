# Harness job skill

Use this when Jason wants a local harness job on the 5090. This skill writes and reads files. It does not open a chat page, does not schedule work, and does not start the GPU.

1. Write the job file over SSH. Do not inline data. The file is `~/harness-inbox/jobs/<id>.json` on the 5090. Fields are only id, step, config, gpu, max_minutes, and nas_out. config is a path that already exists.

2. Read one status file when Jason asks, or when the outbox file changes. The status file is `~/harness-inbox/status/<id>.json`. Do not open a corpus, a manifest, or a batch report.

3. Show the card in plain words. Offer the reading-page path. Do not paste artifacts. If state is queued, say it is waiting. Do not call a queued file a finished pass. A needs-approval card is still waiting. It is not a finished no. The gpu field says idle or busy only after a real probe. Otherwise it says unknown.

4. On needs-approval, ask Jason in the chat he is already in, then write `~/harness-inbox/approve/<id>.json` with decision yes or no, by, and at. Do not start the GPU yourself. A missing file is not a yes.

5. Never open a multi-megabyte corpus file. Never train, export, or seat without an approve file. Never poll on a timer.

A status check is one file read. It is not a cloud-helper turn. If a status check is done by a cloud helper, the local digest pauses new jobs.

Steps the job file may name: gap, decontaminate, train, export, eval, quant, ask, pack-device, run, improve, improve-apply. Anything else is a refuse. train, eval, or run with gpu forbid is a refuse. improve and improve-apply are CPU steps.

READY_FOR_LIVE_TEST stays no. This skill does not flip it.
