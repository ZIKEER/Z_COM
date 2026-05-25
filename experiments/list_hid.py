import hid
devices = hid.enumerate()
print('Found %d HID devices:' % len(devices))
for d in devices:
    print('  VID=0x%04X PID=0x%04X UsagePage=0x%04X Usage=0x%04X Product="%s" Path=%s' % (
        d['vendor_id'], d['product_id'], d['usage_page'], d['usage'],
        d.get('product_string', ''),
        d['path'][:80]))
