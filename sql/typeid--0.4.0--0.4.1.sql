/*
 * No schema changes.
 *
 * 0.4.1 exists only to republish the binaries. The 0.4.0 artifacts were built
 * on Ubuntu 24.04 and required GLIBC_2.38/2.39, so they failed to load on
 * Debian bookworm (glibc 2.36) — the base for cloudnative-pg and the official
 * postgres images — with:
 *
 *   could not load library ".../typeid.so": version `GLIBC_2.38' not found
 *
 * The release now builds on Ubuntu 22.04 (glibc 2.35) and CI refuses to
 * publish a binary requiring anything newer.
 */
