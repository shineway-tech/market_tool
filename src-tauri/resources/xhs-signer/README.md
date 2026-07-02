This directory contains the Xiaohongshu creator-platform request signer used for
direct API calls.

Source reference:
- https://github.com/cv-cat/Spider_XHS
- `static/xhs_creator_260411.js`

The upstream README marks the project as MIT licensed. The repository did not
serve a LICENSE file at the time this asset was added, so keep this note with
the copied signer source.

Local changes:
- Removed the standalone test invocation that logs `window.mnsv2(...)`.
- Runtime `crypto-js` is provided by a small Node.js shim that maps MD5 to the
  built-in `crypto` module.
