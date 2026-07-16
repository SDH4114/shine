# RFC 0003: Scientific memory and devices

Status: accepted architecture, implementation pending.

`Array` and `Tensor` use homogeneous typed buffers with shape, strides, device, and dtype metadata. Slices are views; mutation uses copy-on-write where alias safety requires it. CPU, CUDA, and Metal share one operation contract and dtype-dependent numerical tolerances.

CPU is always available. CUDA is the first Linux/server GPU backend and Metal is the first macOS/Apple Silicon backend. ROCm follows after CUDA and Metal semantics are stable. Arrow is the canonical columnar interchange representation for `Series` and `DataFrame`.
