import os
import torch
from pathlib import Path
from transformers import AutoModel, AutoTokenizer
from onnxruntime.quantization import quantize_dynamic, QuantType

def main():
    model_name = "BAAI/bge-base-en-v1.5"
    out_dir = Path(__file__).parent.parent / "assets" / "model"
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading {model_name}...")
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = AutoModel.from_pretrained(model_name)
    model.eval()

    # Save tokenizer files
    print("Saving tokenizer files...")
    tokenizer.save_pretrained(out_dir)

    # Prepare dummy inputs
    dummy_text = "Search settings..."
    inputs = tokenizer(dummy_text, return_tensors="pt")

    onnx_path = out_dir / "model.onnx"
    print(f"Exporting ONNX model to {onnx_path}...")
    
    # Export with dynamic axes for batch size and sequence length
    torch.onnx.export(
        model,
        args=(inputs["input_ids"], inputs["attention_mask"], inputs["token_type_ids"]),
        f=str(onnx_path),
        input_names=["input_ids", "attention_mask", "token_type_ids"],
        output_names=["last_hidden_state"],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "token_type_ids": {0: "batch_size", 1: "sequence_length"},
            "last_hidden_state": {0: "batch_size", 1: "sequence_length"},
        },
        opset_version=14,
    )

    quant_path = out_dir / "model_int8.onnx"
    print(f"Quantizing model to INT8: {quant_path}...")
    quantize_dynamic(
        model_input=str(onnx_path),
        model_output=str(quant_path),
        weight_type=QuantType.QInt8,
    )

    # Clean up the large float32 model to save space
    if onnx_path.exists():
        print("Removing unquantized float32 ONNX model...")
        os.remove(onnx_path)

    print("Model export and quantization complete!")

if __name__ == "__main__":
    main()
