# Desktop UI v1 Contract

The first DiskTrace desktop screen is a local, image-first recovery workspace. Its purpose is to turn the existing tested command workflow into a clear path for a person who has lost files: select an image, inspect what is available, filter results, understand each result, choose a separate destination, and recover only the selected candidate.

The visual direction is restrained and calm: a dark navy working surface, high-contrast cream text, soft blue information panels, amber review warnings, and a single prominent safe action at each stage. The interface must not use alarming colours or success language when recovery is uncertain. It should say “Recovered — review recommended” or “Recovered and checked,” not “guaranteed.”

The default workflow has four states: `Choose image`, `Review results`, `Inspect details`, and `Save safely`. An empty workspace explains that DiskTrace only reads the selected image. The results list exposes evidence name, method, validation label, size, and source location. Details expose the plain-language explanation and a bounded preview descriptor. Recovery requires an existing destination directory that is distinct from the source image’s storage location; a policy rejection must be shown as an actionable explanation, not a generic error.

The UI shell invokes the existing recovery-core library directly. It never gains a source-write capability, never sends selected images or result content over the network, and never renders recovered binary content. Version one supports opening a typed local image path and a bundled demonstration fixture; native file-picker integration is deferred until the application shell has a verified build on each claimed platform.
