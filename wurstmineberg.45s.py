#!/usr/local/bin/python3

import sys

sys.path.append('/opt/py')

from PIL import Image
import base64
import basedir
import io
import json
import os.path
import requests

CONFIG = basedir.config_dirs('bitbar/plugins/wurstmineberg.json').json()
CACHE = basedir.data_dirs('bitbar/plugin-cache/wurstmineberg/gravatars.json').lazy_json(existing_only=True, default={})

people = requests.get('https://api.wurstmineberg.de/v2/people.json').json()
status = requests.get('https://api.wurstmineberg.de/v2/world/wurstmineberg/status.json').json()

def get_img_str(wmb_id):
    if wmb_id in CACHE:
        return ' image={}'.format(CACHE[wmb_id])
    elif 'gravatar' in people['people'].get(wmb_id, {}):
        r = requests.get(people['people'][wmb_id]['gravatar'])
        i = Image.open(io.BytesIO(r.content)).resize((16,16))
        buf = io.BytesIO()
        i.save(buf, format='PNG')
        CACHE[wmb_id] = base64.b64encode(buf.getvalue()).decode()
        return ' image={}'.format(CACHE[wmb_id])
    else:
        return ''

wurstpick="""iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAMAAAAoLQ9TAAAArlBMVEUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABeyFOlAAAAOXRSTlMABAUHCAkLDBAWFxobHyAhOElUY29yeHl8fX5/iIuNkJelp7a4v8DCxMXHzM7P1+Dh5e3x8vT5/f5sM6tQAAAAiElEQVQY013LxXICAQAE0cYJLtkkENxZluDS//9jOWyhfZtXNUCpUPpdnPZdMgDQuF5UdVePYaTqYTMPkzGE6vEjDdl4f6qem9ybav9Pgzus3FNWWzdYu4OOOnxc6hCokxgCrQFtdQxANRrmAXrqDCABKQC+1GWOp37UTfFZumrEm2xfgO9B5R8QKhPy1xZyawAAAABJRU5ErkJggg=="""

print('{}|templateImage={}'.format(len(status['list']), wurstpick))
print('---')
if CONFIG.get('versionLink', True) is True:
    print('Version: {ver}|href=http://minecraft.gamepedia.com/{ver}'.format(ver=status['version']))
elif CONFIG.get('versionLink', True) == 'alt':
    print('Version: {ver}|color=gray'.format(ver=status['version']))
    print('Version: {ver}|alternate=true href=http://minecraft.gamepedia.com/{ver}'.format(ver=status['version']))
else:
    print('Version: {ver}|color=gray'.format(ver=status['version']))
for wmb_id in status['list']:
    img_str = get_img_str(wmb_id)

    display_name = people['people'].get(wmb_id, {}).get('name', wmb_id)
    if people['people'].get(wmb_id, False) and people['people'][wmb_id].get('slack', False):
        slack_name = people['people'][wmb_id]['slack']['username']
        slack_url = 'https://wurstmineberg.slack.com/messages/@' + slack_name
    else:
        slack_url = None

    if 'favColor' in people['people'].get(wmb_id, {}):
        color = ' color=#{red:02x}{green:02x}{blue:02x}'.format(**people['people'][wmb_id]['favColor'])
    else:
        color = ''
    print('{}|href=https://wurstmineberg.de/people/{}{}{}'.format(display_name, wmb_id, color, img_str))
    if slack_url is not None:
        print('@{}|alternate=true href={} color=red{}'.format(slack_name, slack_url, img_str))

print('---')
print('Start Minecraft|bash=/usr/bin/open param1=-a param2=Minecraft terminal=false')
print('Start TeamSpeak|alternate=true bash=/usr/bin/open param1=-a param2="TeamSpeak 3 Client" terminal=false')
