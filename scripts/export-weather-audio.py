"""Original, deliberately soft synthesized weather beds. No downloaded recordings."""
import math
import random
import struct
import wave
from pathlib import Path

dest=Path(__file__).resolve().parents[1]/'apps/android/game/public/audio'
rate=22050
for kind,seconds in [('rain',16),('wind',20),('thunder',7)]:
    rng=random.Random(481 + len(kind))
    samples=[]
    low=0.
    for i in range(rate*seconds):
        t=i/rate
        white=rng.uniform(-1,1)
        low+= (white-low)*(0.045 if kind=='rain' else 0.006)
        if kind=='rain':
            value=(white*0.14+low*0.6)*(0.85+0.06*math.sin(t*1.2))
        elif kind=='wind':
            value=low*(1.+0.3*math.sin(t*0.65))
        else:
            envelope=(1-math.exp(-t*1.8))*math.exp(-t*0.65)
            value=(low*2.5+0.025*math.sin(t*2*math.pi*48))*envelope
        # End fades guarantee no hard seam; the long beds gently breathe at each loop.
        fade=min(1.,t/0.7,(seconds-t)/0.7)
        samples.append(struct.pack('<h',int(max(-0.5,min(0.5,value*fade))*32767)))
    with wave.open(str(dest/f'garden-{kind}.wav'),'wb') as f:
        f.setnchannels(1);f.setsampwidth(2);f.setframerate(rate);f.writeframes(b''.join(samples))
print('Wrote original soft rain, wind and distant thunder candidates.')
