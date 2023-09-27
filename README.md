# ican

Modern CAN tools written in Rust!

## Install

```
$ git clone https://github.com/nnarain/ican
$ cd ican
$ cargo install --path .
```

## Usage

Drivers can be specified using the syntax: `driver://<opts>`

The default driver is `socketcan` and doesn't require the full specification.

**Dump CAN frames to terminal**

```
ican socketcan://vcan0 dump
```

or

```
ican vcan0 dump
```

**Send CAN frame**

```
ican vcan0 send 123#010203
```

**Send CAN frame at the given rate**

```
ican vcan0 send 123#010203 -r 10
```

**Monitor CAN frames in cansniffer style**

```
ican vcan0 monitor
```
