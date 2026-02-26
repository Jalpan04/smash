use ndarray::Array2;
use ort::{session::Session, value::Tensor};
use tokenizers::Tokenizer;

pub struct SmashAI {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
}

impl SmashAI {
    pub fn new(model_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize ort environment (ignoring if already initialized)
        let _ = ort::init().with_name("smash").commit();

        let encoder_path = format!("{}/encoder_model.onnx", model_dir);
        let decoder_path = format!("{}/decoder_model.onnx", model_dir);
        let tokenizer_path = format!("{}/tokenizer.json", model_dir);

        let encoder = Session::builder()?
            .with_intra_threads(4)?
            .commit_from_file(encoder_path)?;

        let decoder = Session::builder()?
            .with_intra_threads(4)?
            .commit_from_file(decoder_path)?;

        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| e.to_string())?;

        Ok(SmashAI {
            encoder,
            decoder,
            tokenizer,
        })
    }

    pub fn generate(&mut self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let input_text = format!("smash translate: {}", prompt);
        let encoding = self.tokenizer.encode(input_text, true).map_err(|e| e.to_string())?;
        
        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();

        // T5 models exported on CPU often take int64
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();

        let seq_len = input_ids.len();

        let enc_input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids_i64)?;
        let enc_attn_mask_arr = Array2::from_shape_vec((1, seq_len), attention_mask_i64.clone())?;

        let enc_input_ids_val = Tensor::from_array(enc_input_ids_arr)?;
        let enc_attn_mask_val = Tensor::from_array(enc_attn_mask_arr)?;

        // Run Encoder
        // Using positional arguments for standard optimum export 
        // 0 -> input_ids, 1 -> attention_mask
        let enc_outputs = self.encoder.run(ort::inputs![
            "input_ids" => enc_input_ids_val,
            "attention_mask" => enc_attn_mask_val,
        ])?;

        let (shape, data) = enc_outputs["last_hidden_state"].try_extract_tensor::<f32>()?;
        let shape_usize: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
        let hidden_states_arr = ndarray::Array::from_shape_vec(shape_usize, data.to_vec())?;
        
        // Greedy Decoder loop
        let mut decoder_input_ids: Vec<i64> = vec![0]; // T5 pad_token_id is 0
        
        for _ in 0..64 { // Max decoder len
            let dec_seq_len = decoder_input_ids.len();
            let dec_input_ids_arr = Array2::from_shape_vec((1, dec_seq_len), decoder_input_ids.clone())?;
            let dec_input_ids_val = Tensor::from_array(dec_input_ids_arr)?;
            
            let enc_attn_mask_arr_dec = Array2::from_shape_vec((1, seq_len), attention_mask_i64.clone())?;
            let enc_attn_mask_val_dec = Tensor::from_array(enc_attn_mask_arr_dec)?;
            
            let hidden_states_val = Tensor::from_array(hidden_states_arr.clone())?;

            let dec_outputs = self.decoder.run(ort::inputs![
                "input_ids" => dec_input_ids_val,
                "encoder_attention_mask" => enc_attn_mask_val_dec,
                "encoder_hidden_states" => hidden_states_val,
            ])?;

            let (out_shape, out_data) = dec_outputs["logits"].try_extract_tensor::<f32>()?;
            
            // Logits shape is [1, dec_seq_len, vocab_size]
            let vocab_size = out_shape[2] as usize;
            
            let mut best_id = 0;
            let mut best_val = std::f32::NEG_INFINITY;
            
            for v in 0..vocab_size {
                let logit = out_data[(dec_seq_len - 1) * vocab_size + v];
                if logit > best_val {
                    best_val = logit;
                    best_id = v as i64;
                }
            }
            
            if best_id == 1 { // T5 eos_token_id
                break;
            }
            
            decoder_input_ids.push(best_id);
        }

        // Decode output
        let out_u32: Vec<u32> = decoder_input_ids.into_iter().skip(1).map(|x| x as u32).collect();
        let result = self.tokenizer.decode(&out_u32, true).map_err(|e| e.to_string())?;

        Ok(result)
    }
}
