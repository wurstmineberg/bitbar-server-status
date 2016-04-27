#!/usr/local/bin/python3

import os.path
import json
import base64
import requests
from PIL import Image
from io import BytesIO

wurstbit_file = os.path.expanduser('~') + '/.wurstbit-gravatar-cache.json'
if os.path.isfile(wurstbit_file):
    with open(wurstbit_file) as f:
        cache = json.load(f)
else:
    cache = {}

def get_img_str(wmb_id):
    if wmb_id in cache:
        return ' image=' + cache[wmb_id]
    elif 'gravatar' in people['people'].get(wmb_id, {}):
        r = requests.get(people['people'][wmb_id]['gravatar'])
        i = Image.open(BytesIO(r.content))
        i = i.resize((16,16))
        bfr = BytesIO()
        i.save(bfr, format="PNG")
        cache[wmb_id] = base64.b64encode(bfr.getvalue()).decode()
        print(cache)
        with open(wurstbit_file, 'w') as f:
            json.dump(cache, f)
        return ' image=' + cache[wmb_id]
    else:
        return ''

people = requests.get('https://api.wurstmineberg.de/v2/people.json').json()
status = requests.get('https://api.wurstmineberg.de/v2/world/wurstmineberg/status.json').json()

wurstpick="""iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAMAAAAoLQ9TAAAArlBMVEUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABeyFOlAAAAOXRSTlMABAUHCAkLDBAWFxobHyAhOElUY29yeHl8fX5/iIuNkJelp7a4v8DCxMXHzM7P1+Dh5e3x8vT5/f5sM6tQAAAAiElEQVQY013LxXICAQAE0cYJLtkkENxZluDS//9jOWyhfZtXNUCpUPpdnPZdMgDQuF5UdVePYaTqYTMPkzGE6vEjDdl4f6qem9ybav9Pgzus3FNWWzdYu4OOOnxc6hCokxgCrQFtdQxANRrmAXrqDCABKQC+1GWOp37UTfFZumrEm2xfgO9B5R8QKhPy1xZyawAAAABJRU5ErkJggg=="""

print('{}|templateImage={}'.format(len(status['list']), wurstpick))
print('---')
print('Version: {ver}|color=gray'.format(ver=status['version']))
print('Version: {ver}|alternate=true href=http://minecraft.gamepedia.com/{ver}'.format(ver=status['version']))
for wmb_id in status['list']:
    img_str = get_img_str(wmb_id)

    display_name = people['people'].get(wmb_id, {}).get('name', wmb_id)
    if people['people'].get(wmb_id, False) and people['people'][wmb_id].get('slack', False):
        slack_name = people['people'][wmb_id]['slack']['username']
        slack_url = 'https://wurstmineberg.slack.com/messages/@' + slack_name
    else:
        slack_url = None

    if 'favColor' in people['people'].get(wmb_id, {}):
        color = ' color=#' + hex(people['people'][wmb_id]['favColor']['red'])[2:]
        color += hex(people['people'][wmb_id]['favColor']['green'])[2:]
        color += hex(people['people'][wmb_id]['favColor']['blue'])[2:]
    else:
        color = ''
    print('{}|href=https://wurstmineberg.de/people/{}{}{}'.format(display_name, wmb_id, color, img_str))
    if slack_url is not None:
        print('@{}|alternate=true href={} color=red {}'.format(slack_name, slack_url, img_str))

print('---')
print('Start Minecraft | bash=/usr/bin/open param1=-a param2=Minecraft terminal=false')
print('Start TeamSpeak | alternate=true bash=/usr/bin/open param1=-a param2="TeamSpeak 3 Client" terminal=false')
