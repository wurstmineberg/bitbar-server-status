#!/usr/local/bin/python3

import requests

people = requests.get('https://api.wurstmineberg.de/v2/people.json').json()
status = requests.get('https://api.wurstmineberg.de/v2/world/wurstmineberg/status.json').json()

print(len(status['list']))

print('---')
print('Version: {}|color=gray'.format(status['version']))
for wmb_id in status['list']:
    display_name = people['people'].get(wmb_id, {}).get('name', wmb_id)
    print('{}|href=https://wurstmineberg.de/people/{} color=#2889be'.format(display_name, wmb_id))
