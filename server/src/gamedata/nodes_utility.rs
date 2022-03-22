// use rand::prelude::*;

// /// Throw the dice identifier by the text
// pub fn throw_dice(text: String) -> isize {
//     if text.is_empty() {
//         return 0;
//     }

//     let get_char = |index: usize| -> char {
//         let b: u8 = text.as_bytes()[index];
//         b as char
//     };

//     let mut multiplier = 1_isize;
//     let mut dice = 1_isize;
//     let mut modifier = 0_isize;

//     let mut i = 0_usize;

//     if get_char(i).is_ascii_digit() {
//         multiplier = get_char(0).to_digit(10).unwrap() as isize;
//         i += 1;
//     }

//     if i < text.len() {
//         if get_char(i) == 'd' || get_char(i) == 'D' {
//             i += 1;
//             let mut dice_text = "".to_string();
//             while i < text.len() && get_char(i).is_ascii_digit() {
//                 dice_text.push(get_char(i));
//                 i+= 1;
//             }
//             if dice_text.is_empty() == false {
//                 dice = dice_text.parse::<isize>().unwrap();
//             }

//             let mut modifier_text = "".to_string();
//             while i < text.len() {
//                 modifier_text.push(get_char(i));
//                 i+= 1;
//             }
//             if modifier_text.is_empty() == false {
//                 let mod_rc = modifier_text.parse::<isize>();
//                 if mod_rc.is_ok() {
//                     modifier = mod_rc.unwrap();
//                 } else {
//                     return 0;
//                 }
//             }

//             //println!("{}d{}{}", multiplier, dice, modifier);
//             let mut rng = thread_rng();
//             let random = rng.gen_range(1..=dice);

//             let rc = multiplier * random + modifier;
//             return rc;
//         }
//     }
//     0
// }