pub(crate) fn extended_bip32_derivation(
    public_key: &[u8],
    chain_code: &[u8],
    path: &[Vec<u8>],
) -> (Vec<u8>, Vec<u8>) {
    fn secp256k1_decode_point(bytes: &[u8]) -> Option<k256::ProjectivePoint> {
        use k256::elliptic_curve::sec1::FromEncodedPoint;

        match k256::EncodedPoint::from_bytes(bytes) {
            Ok(ept) => {
                let apt = k256::AffinePoint::from_encoded_point(&ept);

                if bool::from(apt.is_some()) {
                    Some(k256::ProjectivePoint::from(apt.unwrap()))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn secp256k1_decode_scalar(bytes: &[u8]) -> Option<k256::Scalar> {
        use k256::elliptic_curve::group::ff::PrimeField;

        if bytes.len() != 32 {
            return None;
        }

        let fb = k256::FieldBytes::from_slice(bytes);
        let s = k256::Scalar::from_repr(*fb);

        if bool::from(s.is_some()) {
            Some(s.unwrap())
        } else {
            None
        }
    }

    fn secp256k1_add(public_key: &[u8], offset: &[u8]) -> Vec<u8> {
        use k256::elliptic_curve::group::GroupEncoding;

        let scalar = secp256k1_decode_scalar(offset).expect("Invalid scalar");

        let public_key = secp256k1_decode_point(public_key).expect("Invalid public key");

        let g = k256::AffinePoint::GENERATOR;
        let g_o = g * scalar;

        let pk_p_g_o = public_key + g_o;

        pk_p_g_o.to_affine().to_bytes().to_vec()
    }

    fn ckdpub(public_key: &[u8], chain_code: &[u8], index: &[u8]) -> (Vec<u8>, Vec<u8>) {
        use hmac::{Hmac, Mac};
        use sha2::Sha512;

        let mut hmac =
            Hmac::<Sha512>::new_from_slice(chain_code).expect("HMAC unable to accept chain code");
        hmac.update(public_key);
        hmac.update(index);
        let hmac_output = hmac.finalize().into_bytes();

        let new_public_key = secp256k1_add(public_key, &hmac_output[..32]);
        let new_chain_code = hmac_output[32..].to_vec();
        (new_public_key, new_chain_code)
    }

    let mut public_key = public_key.to_vec();
    let mut chain_code = if chain_code.is_empty() {
        vec![0; 32]
    } else {
        chain_code.to_vec()
    };

    for idx in path {
        let (new_public_key, new_chain_code) = ckdpub(&public_key, &chain_code, idx);

        public_key = new_public_key;
        chain_code = new_chain_code;
    }

    (public_key, chain_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_bip32_derivation() {
        let master_public_key =
            hex::decode("038cc78aa6040c5f269351939a05aad3a31f86902d0b8cf3085244bb58b6d4337a")
                .unwrap();
        let master_chain_key = vec![];

        let index1 = vec![1, 2, 3, 4, 5];
        let index2 = vec![8, 0, 2, 8, 0, 2];

        let (derived_1_pk, derived_1_cc) =
            extended_bip32_derivation(&master_public_key, &master_chain_key, &[index1.clone()]);

        assert_eq!(
            hex::encode(&derived_1_pk),
            "0216ce1e78a8477d41351c31d0a9f70286935a96bdd5544356d8ecf63a4120979c"
        );
        assert_eq!(
            hex::encode(&derived_1_cc),
            "0811cb2a510b05fedcfb7ba49a5ceb4d48d9ed1210b6a85839e36c53105d3308"
        );

        let (derived_2_pk, derived_2_cc) =
            extended_bip32_derivation(&master_public_key, &master_chain_key, &[index2.clone()]);

        assert_eq!(
            hex::encode(&derived_2_pk),
            "02a9a19dc211db7ec0cbc5883bbc70eedef9d95fed51d950d2fe350e66fbb542aa"
        );
        assert_eq!(
            hex::encode(&derived_2_cc),
            "979ab6baf82d9e4b0793236f61012a48d9b3bfa9b6f30c86a0b5d01c1fab300d"
        );

        let (derived_12_pk, derived_12_cc) =
            extended_bip32_derivation(&master_public_key, &master_chain_key, &[index1, index2]);

        assert_eq!(
            hex::encode(&derived_12_pk),
            "0312ea4418122888ddd95b15261053864861f46f6081a0374c73918c3957b7f35b"
        );
        assert_eq!(
            hex::encode(&derived_12_cc),
            "53ab3ab4ba311976dfae6e7f38fe2131dd5cb72ceff178b06a19b8ad92d1f2d3"
        );
    }
}
