CC = x86_64-w64-mingw32-gcc
CFLAGS = -O2 -g -Wall -Wextra -Werror -ffreestanding -fno-stack-protector -mno-stack-arg-probe
# No C runtime: the import table lists only the Win32/D3D APIs the program really uses.
LDFLAGS = -nostdlib -nostartfiles -Wl,-e,entry -mconsole
LDLIBS = -lkernel32 -luser32 -ld3d11 -ldxgi

all: testgame.exe imports.txt

testgame.exe: testgame.c
	$(CC) $(CFLAGS) $< -o $@ $(LDFLAGS) $(LDLIBS)

imports.txt: testgame.exe
	x86_64-w64-mingw32-objdump -p $< | awk -F'[ \t]+' '/DLL Name:/{dll=$$4} /^\t[0-9a-f]+\t +[0-9]+ +[A-Za-z_]/{print dll"!"$$NF}' > $@

clean:
	rm -f testgame.exe imports.txt
.PHONY: all clean
