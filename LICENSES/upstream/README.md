# Supplemental dependency notices

Some crates omit their workspace-level license files from the published archive.
`dependency-sources.json` maps those exact package versions to the license texts
kept here. Each directory contains `SOURCE.json` with the upstream revision,
download URLs, and SHA-256 hashes of the retrieved files. No crate source or
upstream license terms were changed.

The egui 0.37.0 crates record a VCS commit that is not available in the public
repository. Their workspace license texts are taken from the published 0.37.0
tag; eframe and egui_extras retain their separately recorded source revisions.
The egui code license supplement also accompanies the font crate's own OFL and
Ubuntu Font License files. Enum-map copyright notices are preserved from its
source-file SPDX headers as well as its workspace license texts.

The package script combines these files with the notices present in each crate
and fails if a Windows dependency has neither. This is a notice/provenance record,
not a replacement license or an assertion that darkcr0w owns these components.
