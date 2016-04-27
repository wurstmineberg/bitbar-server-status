#!/usr/local/bin/python3

import requests

people = requests.get('https://api.wurstmineberg.de/v2/people.json').json()
status = requests.get('https://api.wurstmineberg.de/v2/world/wurstmineberg/status.json').json()

print(len(status['list']))

print('---')
print('Version: {ver}|color=gray'.format(ver=status['version']))
print('Version: {ver}|alternate=true href=http://minecraft.gamepedia.com/{ver}'.format(ver=status['version']))
for wmb_id in status['list']:
    display_name = people['people'].get(wmb_id, {}).get('name', wmb_id)
    if people['people'].get(wmb_id, False) and people['people'][wmb_id].get('slack', False):
        slack_name = people['people'][wmb_id]['slack']['username']
        slack_url = 'https://wurstmineberg.slack.com/messages/@' + slack_name
    else:
        slack_url = None
    print('{}|href=https://wurstmineberg.de/people/{} color=#2889be'.format(display_name, wmb_id))
    if slack_url is not None:
        print('@{}|alternate=true href={} color=red'.format(slack_name, slack_url))

print('---')
print('Start Minecraft | bash=/usr/bin/open param1=-a param2=Minecraft terminal=false')
print('Start TeamSpeak | alternate=true bash=/usr/bin/open param1=-a param2="TeamSpeak 3 Client" terminal=false')
