#!/usr/local/bin/python3

import sys

sys.path.append('/opt/py')

import PIL.Image
import base64
import basedir
import io
import requests
import traceback

CONFIG = basedir.config_dirs('bitbar/plugins/wurstmineberg.json').json() or {}
CACHE = basedir.data_dirs('bitbar/plugin-cache/wurstmineberg/avatars.json').lazy_json(existing_only=True, default={})

def get_img_str(uid, zoom=1):
    if str(uid) in CACHE:
        return ' image={}'.format(CACHE[str(uid)])
    else:
        avatar_info = get_json('https://wurstmineberg.de/api/v3/person/{}/avatar.json'.format(uid))
        response = requests.get(avatar_info['url'])
        response.raise_for_status()
        image = PIL.Image.open(io.BytesIO(response.content)).resize((16 * zoom, 16 * zoom), resample=PIL.Image.NEAREST if avatar_info['pixelate'] else PIL.Image.BICUBIC)
        buf = io.BytesIO()
        image.save(buf, format='PNG', dpi=(72 * zoom, 72 * zoom))
        CACHE[str(uid)] = base64.b64encode(buf.getvalue()).decode()
        return ' image={}'.format(CACHE[str(uid)])

def get_json(url):
    response = requests.get(url)
    response.raise_for_status()
    return response.json()

if CONFIG.get('zoom', 1) >= 2:
    WURSTPICK = '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'
else:
    WURSTPICK = 'iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAMAAAAoLQ9TAAAArlBMVEUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABeyFOlAAAAOXRSTlMABAUHCAkLDBAWFxobHyAhOElUY29yeHl8fX5/iIuNkJelp7a4v8DCxMXHzM7P1+Dh5e3x8vT5/f5sM6tQAAAAiElEQVQY013LxXICAQAE0cYJLtkkENxZluDS//9jOWyhfZtXNUCpUPpdnPZdMgDQuF5UdVePYaTqYTMPkzGE6vEjDdl4f6qem9ybav9Pgzus3FNWWzdYu4OOOnxc6hCokxgCrQFtdQxANRrmAXrqDCABKQC+1GWOp37UTfFZumrEm2xfgO9B5R8QKhPy1xZyawAAAABJRU5ErkJggg=='

try:
    people = get_json('https://wurstmineberg.de/api/v3/people.json')
    status = get_json('https://wurstmineberg.de/api/v3/world/wurstmineberg/status.json')
except Exception as e:
    print('?|templateImage={}'.format(WURSTPICK))
    print('---')
    print('{}: {}'.format(e.__class__.__name__, e))
    exc_io = io.StringIO()
    traceback.print_exc(file=exc_io)
    for line in exc_io.getvalue().split('\n'):
        print('--{}'.format(line))
    sys.exit()

if not status['running']:
    if CONFIG.get('showIfOffline', False):
        print('!|templateImage={}'.format(WURSTPICK))
    else:
        print('')
elif len(status['list']) == 0:
    if CONFIG.get('showIfEmpty', False):
        print('|templateImage={}'.format(WURSTPICK))
    else:
        print('')
elif CONFIG.get('singleColor', True) and len(status['list']) == 1 and 'favColor' in people['people'][status['list'][0]]:
    print('1|templateImage={} color=#{red:02x}{green:02x}{blue:02x}'.format(WURSTPICK, **people['people'][status['list'][0]]['favColor']))
else:
    print('{}|templateImage={}'.format(len(status['list']), WURSTPICK))
print('---')
if CONFIG.get('versionLink', True) is True:
    print('Version: {}|href=https://minecraft.gamepedia.com/{}'.format(status['version'], status['version']))
elif CONFIG.get('versionLink', True) == 'alt':
    print('Version: {}'.format(status['version']))
    print('Version: {}|alternate=true href=https://minecraft.gamepedia.com/{} color=blue'.format(status['version'], status['version']))
else:
    print('Version: {}'.format(status['version']))
if not status['running']:
    print('Server offline')
for uid in status.get('list', []):
    img_str = get_img_str(uid, zoom=CONFIG.get('zoom', 1))

    display_name = people['people'].get(str(uid), {}).get('name', str(uid))
    if 'discord' in people['people'].get(str(uid), {}):
        discord_name = people['people'][str(uid)]['discord']['displayName']
        discord_url = 'https://discordapp.com/users/{}/'.format(people['people'][str(uid)]['discord']['snowflake'])
    else:
        discord_url = None

    if 'favColor' in people['people'].get(str(uid), {}):
        color = ' color=#{red:02x}{green:02x}{blue:02x}'.format(**people['people'][str(uid)]['favColor'])
    else:
        color = ''
    print('{}|href=https://wurstmineberg.de/people/{}{}{}'.format(display_name, uid, color, img_str))
    if discord_url is not None:
        print('@{}|alternate=true href={} color=blue{}'.format(discord_name, discord_url, img_str))

print('---')
print('Start Minecraft|bash=/usr/bin/open param1=-a param2=Minecraft terminal=false')
print('Open in Discord|alternate=true href=https://discordapp.com/channels/88318761228054528/388412978677940226 color=blue')
