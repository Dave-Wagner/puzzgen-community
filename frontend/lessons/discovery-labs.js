// Original CC0 interactive examples for the seven discovery workshops.
export const presets = {
 'word-sorting': [{name:'Instruments and flowers',values:['violin tulip flute daisy','tulip','Flowers']},{name:'An unknown item',values:['violin tulip cloud','cloud','Flowers']}],
 'sequence-detective': [{name:'Constant addition',values:['2 5 ? 11 14','addition','8']},{name:'Constant multiplication',values:['2 6 ? 54 162','multiplication','18']},{name:'Consecutive squares',values:['4 9 ? 25 36','squares','16']}],
 'memory-pairs': [{name:'Matching positions',values:['acorn star acorn star','0','2']},{name:'Different faces',values:['leaf moon moon leaf','0','1']}],
 'path-connections': [{name:'Rotate a corner',values:['3','1','12']},{name:'One-sided connection',values:['2','0','1']}],
 futoshiki: [{name:'A completed Latin square',values:['1 2 3 4;2 3 4 1;3 4 1 2;4 1 2 3','r1c1<r1c2;r2c2<r2c3']},{name:'An incomplete grid',values:['0 2 3 4;2 3 4 1;3 4 1 2;4 1 2 3','r1c1<r1c2']}],
 'sound-families': [{name:'Spelling can differ',values:['light','night kite white rain','night kite white']},{name:'Same ending sound',values:['sea','bee tree key cat','bee tree key']}],
 'sentence-assembly': [{name:'Adverb at the end',values:['the curious rabbit rests quietly']},{name:'Adverb at the beginning',values:['quietly the curious rabbit rests']},{name:'A different word order',values:['rabbit the curious rests quietly']}],
};
export const labels = {
 'word-sorting':['Items (space-separated)','Item to classify','Your category'],
 'sequence-detective':['Sequence (one ? for the gap)','Family: addition, multiplication, or squares','Your missing integer'],
 'memory-pairs':['Card faces in positional order','First position (zero-based)','Second position (zero-based)'],
 'path-connections':['Tile port mask (1–15)','Clockwise quarter-turns (0–3)','East neighbor port mask (1–15)'],
 futoshiki:['4×4 grid: spaces between cells, semicolons between rows','Inequalities: r1c1<r1c2; r2c2>r3c2'],
 'sound-families':['Anchor word','Candidate words (space-separated)','Your matches (space-separated)'],
 'sentence-assembly':['Your sentence (five words, without punctuation)'],
};
export const multiline = (kind,index) => (['word-sorting','sequence-detective','memory-pairs','futoshiki','sentence-assembly'].includes(kind)&&index===0)||(kind==='futoshiki'&&index===1)||(kind==='sound-families'&&index>0);
const fail=text=>[{title:'Check your experiment',text}];
const words=s=>s.trim().toLowerCase().split(/\s+/).filter(Boolean);
const known={violin:'Musical instruments',flute:'Musical instruments',trumpet:'Musical instruments',cello:'Musical instruments',tulip:'Flowers',daisy:'Flowers',orchid:'Flowers',daffodil:'Flowers'};
const rhymeGroups=[['/aɪt/','light night kite white'],['/eɪn/','rain train lane chain'],['/æt/','cat hat mat bat'],['/oʊt/','boat coat goat note'],['/iː/','bee tree sea key'],['/uː/','blue shoe true new'],['/ɛl/','bell shell well tell'],['/ɔr/','door floor more shore']];
const rhymes=Object.fromEntries(rhymeGroups.flatMap(([sound,list])=>words(list).map(w=>[w,sound])));
export const rotate=mask=>((mask<<1)&15)|(mask>>3);
const ports=mask=>['north','east','south','west'].filter((_,i)=>mask&(1<<i)).join(', ');
export const builders={
 'word-sorting':values=>{
  const items=words(values[0]);if(!items.length||items.length>20)return fail('Use 1–20 items. The lab has a small reviewed vocabulary, not a full dictionary.');
  const item=values[1].trim().toLowerCase(),guess=values[2].trim().toLowerCase();
  return [{title:'1. Define a category',text:'Musical instruments make music; flowers are blossoms. A category needs a meaning, not merely similar spelling.'},
   {title:'2. Inspect membership',text:'An unknown word is not evidence that it belongs in either category.',rows:[['Item','Reviewed category'],...items.map(w=>[w,known[w]??'Not in the lab vocabulary'])]},
   {title:'3. Group the evidence',text:'Different members share the same rule.',rows:[['Category','Your matching items'],...['Musical instruments','Flowers'].map(c=>[c,items.filter(w=>known[w]===c).join(', ')||'(none)'])]},
   {title:'4. Check your assignment',text:!items.includes(item)?'Your chosen item must appear in the item list.':!known[item]?'This item is outside the lab vocabulary; we cannot verify your assignment.':known[item].toLowerCase()===guess?'Your category matches the reviewed everyday meaning. Explain what feature the members share.':`The reviewed category is ${known[item]}. Consider the meaning rather than the first letter.`}];
 },
 'sequence-detective':values=>{
  const tokens=words(values[0]),family=values[1].trim().toLowerCase();
  if(tokens.length<4||tokens.length>10||tokens.filter(t=>t==='?').length!==1||!['addition','multiplication','squares'].includes(family))return fail('Use 4–10 terms, exactly one ?, and a family of addition, multiplication, or squares.');
  if(tokens.some(t=>t!=='?'&&(!/^-?\d+$/.test(t)||Math.abs(Number(t))>1000000)))return fail('Use integers between -1000000 and 1000000.');
  const gap=tokens.indexOf('?'),visible=tokens.flatMap((t,i)=>t==='?'?[]:[[i,Number(t)]]);const [a,b]=[visible[0],visible.at(-1)];
  let step,base,expected;
  if(family==='addition'){step=(b[1]-a[1])/(b[0]-a[0]);base=a[1]-step*a[0];expected=i=>base+step*i;}
  else if(family==='multiplication'){if(visible.some(([,n])=>n<=0))return fail('This multiplication lab uses positive integers only.');step=Math.pow(b[1]/a[1],1/(b[0]-a[0]));base=a[1]/Math.pow(step,a[0]);expected=i=>base*Math.pow(step,i);}
  else{if(visible.some(([,n])=>n<=0))return fail('Use consecutive positive integer squares.');base=Math.sqrt(a[1])-a[0];if(!Number.isInteger(base)||base<1)return fail('The visible terms do not begin at a positive integer square.');expected=i=>(base+i)**2;}
  if(visible.some(([i,n])=>Math.abs(expected(i)-n)>0.00001)||Math.abs(expected(gap)-Math.round(expected(gap)))>0.00001)return fail('The visible terms do not consistently fit this family with an integer missing value. Change a term or choose another family.');
  const result=Math.round(expected(gap));
  return [{title:'1. State your assumptions',text:`We are working in the ${family} family. Without that constraint, a finite list can fit many invented rules.`,tokens},
   {title:'2. Derive and test a rule',text:family==='squares'?`The roots start at ${base} and increase by one.`:`The constant ${family==='addition'?'difference':'ratio'} is ${Number(step.toFixed(6))}. Test every visible term, not just the first two.`},
   {title:'3. Fill the gap',text:`At zero-based position ${gap}, the rule gives ${result}.`,tokens:tokens.map(t=>t==='?'?String(result):t)},
   {title:'4. Check your prediction',text:/^-?\d+$/.test(values[2].trim())&&Number(values[2])===result?'Your prediction fits the stated family and every visible term.':'Your prediction differs from the result of the stated rule. Check the difference, ratio, or square root.'}];
 },
 'memory-pairs':values=>{
  const deck=words(values[0]);const a=Number(values[1]),b=Number(values[2]);
  if(deck.length<2||deck.length>14||!values.slice(1).every(v=>/^\d+$/.test(v.trim()))||![a,b].every(n=>Number.isInteger(n)&&n>=0&&n<deck.length)||a===b)return fail('Use 2–14 cards and select two different valid zero-based positions.');
  if(deck.some(w=>deck.filter(v=>v===w).length!==2))return fail('Each face must appear exactly twice to model a pair deck.');
  return [{title:'1. Study the model',text:'A position is an identity; a face is a value. Two different identities can share one face. This teaching table deliberately shows the construction data.',rows:[['Position','Face'],...deck.map((v,i)=>[String(i),v])]},
   {title:'2. Turn over two positions',text:`Position ${a} shows ${deck[a]}; position ${b} shows ${deck[b]}.`,tokens:[deck[a],deck[b]]},
   {title:'3. Compare values, not positions',text:deck[a]===deck[b]?'The faces match. Keep these two distinct cards paired.':'The faces differ. Keep their locations in mind and try a different position.'},
   {title:'4. Complete the invariant',text:'A full solution uses each card identity exactly once. Matching one pair is only partial progress. The API checks all pairs; the interface hides and reveals faces.'}];
 },
 'path-connections':values=>{
  const [mask,turns,neighbor]=values.map(Number);if(!values.every(v=>/^\d+$/.test(v.trim()))||![mask,neighbor].every(v=>Number.isInteger(v)&&v>=1&&v<=15)||!Number.isInteger(turns)||turns<0||turns>3)return fail('Use masks 1–15 and an integer number of clockwise quarter-turns from 0 to 3.');
  let result=mask;for(let i=0;i<turns;i++)result=rotate(result);const east=Boolean(result&2),west=Boolean(neighbor&8);
  return [{title:'1. Decode a bit mask',text:'North=1, east=2, south=4, west=8. Add the active port bits to describe a tile.',tokens:[`${mask}: ${ports(mask)}`]},
   {title:'2. Rotate without changing the piece',text:`After ${turns} clockwise quarter-turn(s), mask ${mask} becomes ${result}. The number of ports is unchanged.`,tokens:[ports(result)]},
   {title:'3. Check one shared edge',text:east&&west?'The tile’s east port meets the neighbor’s west port.':east!==west?'A one-sided port leaks: a connection must exist on both sides.':'Neither tile has a port on this shared edge, so this edge is sealed.',rows:[['Side','Has a port?'],['Tile east',String(east)],['Neighbor west',String(west)]]},
   {title:'4. Check the whole network',text:'A compatible edge is not a complete solution. The API checks every boundary, every opposite-port pair, and a graph traversal reaching every tile from the source.'}];
 },
 futoshiki:values=>{
  const rows=values[0].trim().split(';').map(row=>words(row).map(Number));if(rows.length!==4||rows.some(row=>row.length!==4||row.some(v=>!Number.isInteger(v)||v<0||v>4)))return fail('Enter four semicolon-separated rows of four digits (0–4). Zero marks an empty cell.');
  const clues=values[1].trim().split(';').map(s=>s.trim()).filter(Boolean);if(clues.length>16)return fail('Use at most 16 inequalities.');
  const parsed=clues.map(s=>s.match(/^r([1-4])c([1-4])\s*([<>])\s*r([1-4])c([1-4])$/i));if(parsed.some(p=>!p))return fail('Write inequalities like r1c1<r1c2; r2c2>r3c2.');
  const describe=line=>{const used=line.filter(v=>v!==0);return new Set(used).size!==used.length?'Repeated number':used.length===4?'Complete and distinct':'Incomplete, no repetition yet';};
  const checks=parsed.map(p=>{const x=rows[+p[1]-1][+p[2]-1],y=rows[+p[4]-1][+p[5]-1];return [p[0],!x||!y?'Not yet decided':(p[3]==='<'?x<y:x>y)?'Satisfied':'Violated'];});
  const cols=rows[0].map((_,c)=>rows.map(r=>r[c]));
  return [{title:'1. Check rows',text:'Use 1–4 once per row. A zero is unknown, not a fifth symbol.',rows:[['Row','Cells','Status'],...rows.map((row,i)=>[String(i+1),row.join(' '),describe(row)])]},
   {title:'2. Check columns',text:'Apply the same rule vertically. Futoshiki has no Sudoku box constraint.',rows:[['Column','Cells','Status'],...cols.map((col,i)=>[String(i+1),col.join(' '),describe(col)])]},
   {title:'3. Check inequalities',text:'The pointed side of < faces the smaller number. An unknown endpoint cannot decide a clue yet.',rows:[['Clue','Status'],...checks]},
   {title:'4. Combine constraints',text:rows.every(r=>describe(r)==='Complete and distinct')&&cols.every(c=>describe(c)==='Complete and distinct')&&checks.every(c=>c[1]==='Satisfied')?'This grid satisfies the lab constraints. The API additionally preserves its givens and verifies generated puzzles have one completion.':'This grid is incomplete or violates a constraint. Edit a cell and inspect the row, column and inequality evidence again.'}];
 },
 'sound-families':values=>{
  const anchor=values[0].trim().toLowerCase(),candidates=words(values[1]),selected=words(values[2]);if(!rhymes[anchor]||!candidates.length||candidates.length>20||new Set(candidates).size!==candidates.length)return fail('Choose an anchor from a preset family and 1–20 distinct candidates. The lab uses a small reviewed US-English pronunciation table.');
  const expected=candidates.filter(w=>rhymes[w]===rhymes[anchor]);const unknown=candidates.some(w=>!rhymes[w]);
  return [{title:'1. Hear the anchor',text:`The reviewed ending of “${anchor}” is ${rhymes[anchor]}. The ending means the stressed vowel plus following sounds, not just the last written letter.`},
   {title:'2. Compare sounds',text:'Unknown pronunciations remain unknown; they are not automatically non-rhymes.',rows:[['Candidate','Ending','Evidence'],...candidates.map(w=>[w,rhymes[w]??'Unknown',!rhymes[w]?'Not reviewed':rhymes[w]===rhymes[anchor]?'Rhymes':'Different ending'])]},
   {title:'3. Collect the reviewed matches',text:'Spelling can differ while the sound agrees.',tokens:expected},
   {title:'4. Check your selection',text:unknown?'At least one candidate lacks a reviewed pronunciation, so this lab cannot certify a complete selection.':new Set(selected).size===selected.length&&selected.length===expected.length&&selected.every(w=>expected.includes(w))?'You selected every reviewed match exactly once. Accents can differ; the declared convention matters.':'Your selection is missing a reviewed match or includes a different ending. Compare the sound table.'}];
 },
 'sentence-assembly':values=>{
  const line=words(values[0]);const groups=[['article',['the']],['adjective',['small','curious','sleepy','brave','gentle','cheerful']],['animal',['rabbit','fox','robin','turtle','mouse','cat','otter','badger']],['verb',['rests','waits','wanders','plays','moves']],['adverb',['quietly','happily','gently','calmly','slowly','patiently']]];
  if(line.length!==5||groups.some(([,set])=>line.filter(w=>set.includes(w)).length!==1))return fail('Use five words with one article, adjective, animal, verb and adverb from the preset vocabulary. This is a controlled grammar, not a general English parser.');
  const parts=groups.map(([,set])=>line.find(w=>set.includes(w)));const orders=[[0,1,2,3,4],[0,1,2,4,3],[4,0,1,2,3]].map(order=>order.map(i=>parts[i]).join(' '));
  return [{title:'1. Identify the roles',text:'Meaningful order begins with roles, not random permutations.',rows:[['Role','Your word'],...groups.map(([role],i)=>[role,parts[i]])]},
   {title:'2. Build a noun phrase',text:`“${parts.slice(0,3).join(' ')}” names and describes the actor. Keep article, adjective and noun together.`,tokens:parts.slice(0,3)},
   {title:'3. Move the adverb',text:'In this controlled grammar, the adverb can come last, before the verb, or before the whole noun phrase.',rows:[['Accepted order'],...orders.map(s=>[s])]},
   {title:'4. Check this construction',text:orders.includes(line.join(' '))?'Your order fits one of the three reviewed constructions. Other English sentences may be grammatical outside this small model.':'Your order is outside these constructions. Keep the noun phrase together and move only the adverb to a supported position.'}];
 },
};
