#!/usr/local/bin/python3

import os
import sys
import os.path
import json
import base64
import requests
from PIL import Image
from io import BytesIO
from collections import defaultdict

wurstbit_dir = os.path.expanduser('~') + '/.config/wmb-bitbar/'
gravatar_file = 'gravatar_cache.json'
if not os.path.isdir(wurstbit_dir):
    os.makedirs(wurstbit_dir)
if os.path.isfile(wurstbit_dir + gravatar_file):
    with open(wurstbit_dir + gravatar_file) as f:
        cache = json.load(f)
else:
    cache = {}

def get_img_str(wmb_id):
    if wmb_id in cache:
        return ' image=' + cache[wmb_id]
    elif 'gravatar' in people['people'].get(wmb_id, {}):
        try:
            r = requests.get(people['people'][wmb_id]['gravatar'])
        except:
            return ''  # don't fail if we can't get an image
        i = Image.open(BytesIO(r.content))
        i = i.resize((16,16))
        bfr = BytesIO()
        i.save(bfr, format="PNG")
        cache[wmb_id] = base64.b64encode(bfr.getvalue()).decode()
        with open(wurstbit_dir + gravatar_file, 'w') as f:
            json.dump(cache, f)
        return ' image=' + cache[wmb_id]
    else:
        skinsurl = 'http://skins.minecraft.net/MinecraftSkins/{}.png'
        try:
            r = requests.get(skinsurl.format(people['people'][wmb_id]['minecraft']['nicks'][-1]))
        except:
            return ''  # don't fail if we can't get an image
        i = Image.open(BytesIO(r.content))
        i = i.crop((8,8,16,16)).resize((16,16))
        bfr = BytesIO()
        i.save(bfr, format="PNG")
        cache[wmb_id] = base64.b64encode(bfr.getvalue()).decode()
        with open(wurstbit_dir + gravatar_file, 'w') as f:
            json.dump(cache, f)
        return ' image=' + cache[wmb_id]

message = '''{num}|templateImage=iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAMAAAAoLQ9TAAAArlBMVEUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABeyFOlAAAAOXRSTlMABAUHCAkLDBAWFxobHyAhOElUY29yeHl8fX5/iIuNkJelp7a4v8DCxMXHzM7P1+Dh5e3x8vT5/f5sM6tQAAAAiElEQVQY013LxXICAQAE0cYJLtkkENxZluDS//9jOWyhfZtXNUCpUPpdnPZdMgDQuF5UdVePYaTqYTMPkzGE6vEjDdl4f6qem9ybav9Pgzus3FNWWzdYu4OOOnxc6hCokxgCrQFtdQxANRrmAXrqDCABKQC+1GWOp37UTfFZumrEm2xfgO9B5R8QKhPy1xZyawAAAABJRU5ErkJggg=={numcolor}
---
{playerlist}
---
Start Minecraft | bash=/usr/bin/open param1=-a param2=Minecraft terminal=false
Start TeamSpeak | alternate=true bash=/usr/bin/open param1=-a param2="TeamSpeak 3 Client" terminal=false
{versioninfo}
{detailinfo}'''

versioninfo = """Version: {ver}
Version: {ver}|alternate=true href=http://minecraft.gamepedia.com/{ver}'"""

detailinfo = """{time}, {weather}"""

mappings = defaultdict(str)

try:
    people = requests.get('https://api.wurstmineberg.de/v2/people.json').json()
    status = requests.get('https://api.wurstmineberg.de/v2/world/wurstmineberg/status.json').json()
    level = requests.get('https://api.wurstmineberg.de/v2/world/wurstmineberg/level.json').json()
except:  # for some reason not reachable
    mappings['num'] = '?'
    mappings['playerlist'] = 'No internet connection|color=gray'
    print(message.format(**mappings))
    sys.exit(0)


mappings['num'] = len(status['list'])

mappings['versioninfo'] = versioninfo.format(ver=status['version'])

if level['Data']['thundering']:
    weather = ":zap: Thunderstorm"
else:
    weather = ":umbrella: Rain" if level['Data']['raining'] else ":sunny: Clear"

ticks = (level['Data']['DayTime']+6000) % 24000
hour, minutes = ticks//1000, int((ticks % 1000)/1000*60)
clock = str(hour%12 if hour%12 else 12) + ("30" if minutes >= 30 else "")

mappings['detailinfo'] = detailinfo.format(
        time=':clock{}: {}:{:02d}'.format(clock, hour, minutes),
        weather=weather,
        )

playerlist = ""
for wmb_id in status['list']:
    img_str = get_img_str(wmb_id)

    display_name = people['people'].get(wmb_id, {}).get('name', wmb_id)
    if people['people'].get(wmb_id, False) and people['people'][wmb_id].get('slack', False):
        slack_name = people['people'][wmb_id]['slack']['username']
        slack_url = 'https://wurstmineberg.slack.com/messages/@' + slack_name
    else:
        slack_url = None

    if 'favColor' in people['people'].get(wmb_id, {}):
        color = ' color=#%02x%02x%02x' % (
                people['people'][wmb_id]['favColor']['red'],
                people['people'][wmb_id]['favColor']['green'],
                people['people'][wmb_id]['favColor']['blue'],
                )
        if mappings['num'] == 1:
            mappings['numcolor'] = color
    else:
        color = ''
    playerlist += '{}|href=https://wurstmineberg.de/people/{}{}{}\n'.format(display_name, wmb_id, color, img_str)
    if slack_url is not None:
        playerlist += '@{}|alternate=true href={} color=red {}\n'.format(slack_name, slack_url, img_str)
mappings['playerlist'] = playerlist

print(message.format(**mappings))
