all: csh rsh

csh: sh.c
	gcc-4.9 -g -fdiagnostics-color -o $@ $<

rsh: sh.rs
	rustc -o $@ $<
