(function() {var implementors = {};
implementors["ecdsa"] = [{"text":"impl&lt;C, D&gt; <a class=\"trait\" href=\"signature/signer/trait.RandomizedDigestSigner.html\" title=\"trait signature::signer::RandomizedDigestSigner\">RandomizedDigestSigner</a>&lt;D, <a class=\"struct\" href=\"ecdsa/struct.Signature.html\" title=\"struct ecdsa::Signature\">Signature</a>&lt;C&gt;&gt; for <a class=\"struct\" href=\"ecdsa/struct.SigningKey.html\" title=\"struct ecdsa::SigningKey\">SigningKey</a>&lt;C&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;C: <a class=\"trait\" href=\"ecdsa/trait.Curve.html\" title=\"trait ecdsa::Curve\">Curve</a> + <a class=\"trait\" href=\"elliptic_curve/order/trait.Order.html\" title=\"trait elliptic_curve::order::Order\">Order</a> + <a class=\"trait\" href=\"elliptic_curve/point/trait.ProjectiveArithmetic.html\" title=\"trait elliptic_curve::point::ProjectiveArithmetic\">ProjectiveArithmetic</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;D: <a class=\"trait\" href=\"digest/fixed/trait.FixedOutput.html\" title=\"trait digest::fixed::FixedOutput\">FixedOutput</a>&lt;OutputSize = C::<a class=\"type\" href=\"elliptic_curve/trait.Curve.html#associatedtype.FieldSize\" title=\"type elliptic_curve::Curve::FieldSize\">FieldSize</a>&gt; + <a class=\"trait\" href=\"digest/trait.BlockInput.html\" title=\"trait digest::BlockInput\">BlockInput</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> + <a class=\"trait\" href=\"digest/trait.Reset.html\" title=\"trait digest::Reset\">Reset</a> + <a class=\"trait\" href=\"digest/trait.Update.html\" title=\"trait digest::Update\">Update</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"type\" href=\"elliptic_curve/scalar/type.Scalar.html\" title=\"type elliptic_curve::scalar::Scalar\">Scalar</a>&lt;C&gt;: <a class=\"trait\" href=\"ff/trait.PrimeField.html\" title=\"trait ff::PrimeField\">PrimeField</a>&lt;Repr = <a class=\"type\" href=\"elliptic_curve/type.FieldBytes.html\" title=\"type elliptic_curve::FieldBytes\">FieldBytes</a>&lt;C&gt;&gt; + <a class=\"trait\" href=\"ecdsa/hazmat/trait.FromDigest.html\" title=\"trait ecdsa::hazmat::FromDigest\">FromDigest</a>&lt;C&gt; + <a class=\"trait\" href=\"elliptic_curve/ops/trait.Invert.html\" title=\"trait elliptic_curve::ops::Invert\">Invert</a>&lt;Output = <a class=\"type\" href=\"elliptic_curve/scalar/type.Scalar.html\" title=\"type elliptic_curve::scalar::Scalar\">Scalar</a>&lt;C&gt;&gt; + <a class=\"trait\" href=\"ecdsa/hazmat/trait.SignPrimitive.html\" title=\"trait ecdsa::hazmat::SignPrimitive\">SignPrimitive</a>&lt;C&gt; + <a class=\"trait\" href=\"zeroize/trait.Zeroize.html\" title=\"trait zeroize::Zeroize\">Zeroize</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"type\" href=\"ecdsa/type.SignatureSize.html\" title=\"type ecdsa::SignatureSize\">SignatureSize</a>&lt;C&gt;: <a class=\"trait\" href=\"generic_array/trait.ArrayLength.html\" title=\"trait generic_array::ArrayLength\">ArrayLength</a>&lt;u8&gt;,&nbsp;</span>","synthetic":false,"types":["ecdsa::sign::SigningKey"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()