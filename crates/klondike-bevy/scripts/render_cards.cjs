// Authoring only: npm install --prefix this directory, then node render_cards.cjs.
const sharp = require('sharp');
const path = require('node:path');
const assets = path.resolve(__dirname, '../assets');
const width = 320, height = 448;
async function main() {
  const cards = [];
  for (const suit of ['C','D','H','S']) {
    for (const rank of ['A','2','3','4','5','6','7','8','9','10','J','Q','K']) {
      const i = cards.length;
      const input = await sharp(path.join(assets, 'source', rank+suit+'.svg'), {density: 192})
        .resize(width,height).flatten({background:'#ffffff'}).png().toBuffer();
      cards.push({input,left:(i%8)*width,top:Math.floor(i/8)*height});
    }
  }
  await sharp({create:{width:width*8,height:height*7,channels:3,background:'#ffffff'}})
    .composite(cards).png().toFile(path.join(assets,'cards.png'));
  await sharp(path.join(assets,'source/back.svg'),{density:192}).resize(width,height)
    .flatten({background:'#ffffff'}).png().toFile(path.join(assets,'back.png'));
  console.log('Rendered classic card atlas and back.');
}
main().catch(error=>{console.error(error);process.exitCode=1});
