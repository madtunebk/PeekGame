#ifndef STEAM_BRIDGE_H
#define STEAM_BRIDGE_H
#include <stdint.h>

/* Windows x64 steamclient64.dll exports -> the real Linux steamclient.so.
 * Nothing here authenticates or answers on Steam's behalf: every query is forwarded to
 * the running Linux Steam client, and anything not bridged stops or fails visibly. */

/* Loads the Linux client. NULL uses $STEAM_BRIDGE_CLIENT, then ~/.steam/sdk64/steamclient.so.
 * Returns 0 on success. Exports also call this lazily. */
int steam_bridge_open(const char *path);

/* Windows-ABI (ms_abi) address for a steamclient64.dll export, or NULL if it is not bridged. */
void *steam_bridge_export(const char *name);

/* PID of the running Linux Steam client from ~/.steam/steam.pid, or 0 if it is not running.
 * This is the honest source for HKCU\Software\Valve\Steam\ActiveProcess "pid". */
uint32_t steam_bridge_steam_pid(void);

/* Returns 1 if addr belongs to steamclient64.dll exports, vtables, wrappers, or the Linux client. */
int steam_bridge_contains_address(const void *addr);

#endif
