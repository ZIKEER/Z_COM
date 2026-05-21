class DataHandler:
    CONTROL_CHAR_MAP = {
        0x00: '\u2400', 0x01: '\u2401', 0x02: '\u2402', 0x03: '\u2403',
        0x04: '\u2404', 0x05: '\u2405', 0x06: '\u2406', 0x07: '\u2407',
        0x08: '\u2408', 0x09: '\u2409', 0x0A: '\u240A', 0x0B: '\u240B',
        0x0C: '\u240C', 0x0D: '\u240D', 0x0E: '\u240E', 0x0F: '\u240F',
        0x10: '\u2410', 0x11: '\u2411', 0x12: '\u2412', 0x13: '\u2413',
        0x14: '\u2414', 0x15: '\u2415', 0x16: '\u2416', 0x17: '\u2417',
        0x18: '\u2418', 0x19: '\u2419', 0x1A: '\u241A', 0x1B: '\u241B',
        0x1C: '\u241C', 0x1D: '\u241D', 0x1E: '\u241E', 0x1F: '\u241F',
        0x7F: '\u2421',
    }

    @staticmethod
    def bytes_to_hex(data):
        return ' '.join(f'{b:02X}' for b in data)

    @staticmethod
    def bytes_to_ascii(data):
        result = []
        for b in data:
            if b in DataHandler.CONTROL_CHAR_MAP:
                result.append(DataHandler.CONTROL_CHAR_MAP[b])
                if b == 0x0A:
                    result.append('\n')
            elif 0x20 <= b < 0x7F:
                result.append(chr(b))
            else:
                result.append(f'\\x{b:02x}')
        return ''.join(result)

    @staticmethod
    def format_display(data, mode):
        if mode == 'HEX':
            return DataHandler.bytes_to_hex(data)
        elif mode == 'ASCII':
            return DataHandler.bytes_to_ascii(data)
        elif mode == 'MIXED':
            hex_str = DataHandler.bytes_to_hex(data)
            ascii_str = DataHandler.bytes_to_ascii(data)
            return f"\n \u2190 HEX: {hex_str}\n \u2190 ASCII: {ascii_str}"
        return ''

    @staticmethod
    def validate_hex_input(text):
        hex_str = text.replace(' ', '').replace('\n', '')
        try:
            bytes.fromhex(hex_str)
            return True
        except ValueError:
            return False
