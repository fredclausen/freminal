# Supported Control Codes

## Key

- ⬜️ - Not implemented yet
- ✅ - Implemented
- 🚧 - Partially implemented
- ❌ - Will not be implemented

## Standard Escape Codes

| Control Code | Name                   | Description                                                                                | Implemented |
| ------------ | ---------------------- | ------------------------------------------------------------------------------------------ | ----------- |
| ESC SP F     | 7 Bit Control          |                                                                                            | ❌          |
| ESC SP G     | 8 Bit Control          |                                                                                            | ❌          |
| ESC SP L     | Ansi Conformance Level | Level 1                                                                                    | ❌          |
| ESC SP M     | Ansi Conformance Level | Level 2                                                                                    | ❌          |
| ESC SP N     | Ansi Conformance Level | Level 3                                                                                    | ❌          |
| ESC # 3      | DECDHL                 | Double Line Height, Top Half                                                               | ❌          |
| ESC # 4      | DECDHL                 | Double Line Height, Bottom Half                                                            | ❌          |
| ESC # 5      | DECSWL                 | Single Width Line                                                                          | ❌          |
| ESC # 6      | DECDWL                 | Double Width Line                                                                          | ❌          |
| ESC # 8      | DECALN                 | Screen Alignment Test                                                                      | ⬜          |
| ESC % @      | Character Set          | Default Character Set                                                                      | ❌          |
| ESC % G      | Character Set          | UTF Character Set                                                                          | ❌          |
| ESC ( C      | Character Set          | G0 Character Set                                                                           | ❌          |
| ESC ) C      | Character Set          | G1 Character Set                                                                           | ❌          |
| ESC \* C     | Character Set          | G2 Character Set                                                                           | ❌          |
| ESC + C      | Character Set          | Where `C` is a charset defined at [xfreeorg](https://www.xfree86.org/current/ctlseqs.html) | ❌          |
| ESC 7        | Save Cursor            |                                                                                            | ⬜          |
| ESC 8        | Restore Cursor         |                                                                                            | ⬜          |
| ESC =        | DECPAM                 | Application Keypad Mode                                                                    | ✅          |
| ESC >        | DECPNM                 | Application Normal Keypad Mode                                                             | ✅          |
| ESC F        |                        | Cursor to lower left of screen                                                             | ❌          |
| ESC c        |                        | Full reset (RIS)                                                                           | ❌          |
| ESC l        |                        | Memory lock                                                                                | ❌          |
| ESC m        |                        | Memory unlock                                                                              | ❌          |
| ESC n        | Character Set          | Invoke the G2 character set as GL                                                          | ❌          |
| ESC o        | Character Set          | Invoke the G3 character set as GL                                                          | ❌          |
| ESC \|       | Character Set          | Invoke the G3 character set as GR                                                          | ❌          |
| ESC }        | Character Set          | Invoke the G2 character set as GR                                                          | ❌          |
| ESC ~        | Character Set          | Invoke the G1 character set as GR                                                          | ❌          |

## CSI Control Codes

| Control Code | Name | Description                                            | Implemented |
| ------------ | ---- | ------------------------------------------------------ | ----------- |
| CSI Ps D     | CUB  | Cursor Backward [Ps] (default = 1)                     | ✅          |
| CSI Ps G     | CHA  | Cursor Character Absolute [column] (default = [row,1]) | ✅          |
| CSI Ps m     | SGR  | See [SGR](/Documents/SGR.md)                           | ✅          |
