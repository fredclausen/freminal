# Supported Control Codes

## Key

- ⬜️ - Not implemented yet
- ✅ - Implemented
- 🚧 - Partially implemented
- ❌ - Will not be implemented

## Standard Escape Codes

| Control Code | Name                   | Description                     | Implemented |
| ------------ | ---------------------- | ------------------------------- | ----------- |
| ESC SP F     | 7 Bit Control          |                                 | ❌          |
| ESC SP G     | 8 Bit Control          |                                 | ❌          |
| ESC SP L     | Ansi Conformance Level | Level 1                         | ❌          |
| ESC SP M     | Ansi Conformance Level | Level 2                         | ❌          |
| ESC SP N     | Ansi Conformance Level | Level 3                         | ❌          |
| ESC # 3      | DECDHL                 | Double Line Height, Top Half    | ❌          |
| ESC # 4      | DECDHL                 | Double Line Height, Bottom Half | ❌          |
| ESC # 5      | DECSWL                 | Single Width Line               | ❌          |
| ESC # 6      | DECDWL                 | Double Width Line               | ❌          |
| ESC # 8      | DECALN                 | Screen Alignment Test           | ⬜          |
| ESC % @      | Character Set          | Default Character Set           | ❌          |
| ESC % G      | Character Set          | UTF Character Set               | ❌          |
| ESC ( C      | Character Set          | G0 Character Set                | ❌          |
| ESC ) C      | Character Set          | G1 Character Set                | ❌          |
| ESC \* C     | Character Set          | G2 Character Set                | ❌          |

## CSI Control Codes

| Control Code | Name | Description                                            | Implemented |
| ------------ | ---- | ------------------------------------------------------ | ----------- |
| CSI Ps D     | CUB  | Cursor Backward [Ps] (default = 1)                     | ✅          |
| CSI Ps G     | CHA  | Cursor Character Absolute [column] (default = [row,1]) | ✅          |
| CSI Ps m     | SGR  | See [SGR](/Documents/SGR.md)                           | ✅          |
