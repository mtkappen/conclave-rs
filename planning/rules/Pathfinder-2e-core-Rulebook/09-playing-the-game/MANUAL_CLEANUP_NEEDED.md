# Chapter 9: Manual Cleanup Required

## Status Summary

**Extraction Complete**: Raw content extracted from lines 26069-28555 (~2,486 lines)
**Files Created**: 6 section files + full cleaned text (179KB total)
**Quality Issue**: Severe two-column PDF layout mixing requires manual review

## Problems Identified

### 1. Sidebar/Navigation Text Insertion
Right column navigation elements appear mid-sentence:
- "Crafting & Treasure" inserted into paragraphs
- "Game mastering" appearing in text flow
- Individual words like "Feats", "Equipment", "Spells" breaking sentences

### 2. Column Interleaving
Text from right column is interleaved with left column at word level:
Example: "determine in every mode of play. The pace of your adventure and the specific rules you'll use"
Should be: "determine the pace of your adventure..."

### 3. Garbled Headers
Some section headers have corrupted text:
- "M a p ultiple ttaCk enalty" (Multiple Attack Penalty)
- "R p ange enalty" (Range Penalty)

## Files Created

1. **00-cleaned-full.txt** (179KB) - Complete cleaned chapter text
2. **01-introduction.md** (805 bytes) - Chapter overview
3. **02-making-choices.md** (16KB) - Making Choices section
4. **03-general-rules.md** (112KB) - General rules, checks, damage
5. **04-encounter-mode.md** (49KB) - Combat and encounter rules
6. **05-exploration-mode.md** (9.7KB) - Exploration activities
7. **06-downtime-mode.md** (4.5KB) - Downtime activities

## Recommended Cleanup Approach

### Option 1: Manual Review (Recommended)
1. Open `00-cleaned-full.txt` in a text editor
2. Read through and manually fix:
   - Remove sidebar words inserted mid-sentence
   - Fix interleaved column text
   - Repair garbled headers
3. Split cleaned content back into section files

### Option 2: Skip for Now
Mark Chapter 9 as "Needs Manual Cleanup" in PROGRESS.md and proceed to Chapters 10-11
(They may have similar issues)

### Option 3: Re-extract from Source
If you have access to the original PDF, consider:
- Using better PDF-to-text conversion tools
- Extracting with column-aware parsing
- Using OCR software that understands layout

## Comparison with Earlier Chapters

Chapters 1-8 were extracted cleanly because:
- They had less severe two-column mixing
- Sidebar text was easier to identify and remove
- Content structure was more consistent

Chapter 9 has worse issues because:
- More complex rules content with tables and sidebars
- Denser text layout in source PDF
- More navigation elements interspersed throughout

## Next Steps

1. **Decide on cleanup approach** (manual vs skip vs re-extract)
2. **If manual**: Allocate ~2-4 hours for thorough review
3. **If skip**: Update PROGRESS.md and continue to Chapter 10
4. **Document lessons learned** for future chapter extraction

## Notes

Extraction attempted: July 22, 2026
Automated cleaning applied: Multiple iterations
Remaining manual work: Significant (text mixing at word level)
