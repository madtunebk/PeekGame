CC ?= cc
CFLAGS = -O2 -g -Wall -Wextra -Werror -fPIC
LDLIBS = -ldl -pthread

all: steam_bridge.o fake_steamclient.so test_bridge host_probe

steam_bridge.o: steam_bridge.c steam_bridge.h

fake_steamclient.so: fake_steamclient.c
	$(CC) $(CFLAGS) -shared $< -o $@

test_bridge: test_bridge.c steam_bridge.o
	$(CC) $(CFLAGS) $^ $(LDLIBS) -o $@

host_probe: host_probe.c steam_bridge.o
	$(CC) $(CFLAGS) $^ $(LDLIBS) -o $@

test: test_bridge fake_steamclient.so
	./test_bridge $(CURDIR)/fake_steamclient.so

clean:
	rm -f steam_bridge.o fake_steamclient.so test_bridge host_probe
.PHONY: all clean test
