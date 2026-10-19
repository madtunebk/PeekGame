//! Export declarations for security. Implementation stays in api/.
use crate::api::system::pointers;
use crate::api::system::random;
use crate::api::system::security;
use crate::registry::Export;

exports! {
    export!(
        "advapi32.dll",
        "BuildTrusteeWithSidW",
        "security",
        "implemented",
        "initializes TRUSTEE_W with caller-owned SID",
        security::build_trustee_with_sid_w
    ),
    export!(
        "advapi32.dll",
        "SetEntriesInAclW",
        "security",
        "partial",
        "constructs SID-based ACL from explicit entries and an empty old ACL",
        security::set_entries_in_acl_w
    ),
    export!(
        "advapi32.dll",
        "OpenProcessToken",
        "security",
        "partial",
        "current process, query access",
        security::open_process_token
    ),
    export!(
        "advapi32.dll",
        "OpenThreadToken",
        "security",
        "implemented",
        "no impersonation: ERROR_NO_TOKEN",
        security::open_thread_token
    ),
    export!(
        "advapi32.dll",
        "GetTokenInformation",
        "security",
        "partial",
        "elevation and integrity level (root is elevated)",
        security::token_information
    ),
    export!(
        "advapi32.dll",
        "GetSidSubAuthority",
        "security",
        "implemented",
        "SID layout",
        security::sid_sub_authority
    ),
    export!(
        "advapi32.dll",
        "GetSidSubAuthorityCount",
        "security",
        "implemented",
        "SID layout",
        security::sid_sub_authority_count
    ),
    export!(
        "advapi32.dll",
        "IsValidSid",
        "security",
        "implemented",
        "validates revision and sub-authority count",
        security::is_valid_sid
    ),
    export!(
        "advapi32.dll",
        "GetLengthSid",
        "security",
        "implemented",
        "header and sub-authority byte size",
        security::get_length_sid
    ),
    export!(
        "advapi32.dll",
        "CopySid",
        "security",
        "implemented",
        "copies validated SID buffer",
        security::copy_sid
    ),
    export!(
        "advapi32.dll",
        "SystemFunction036",
        "security",
        "implemented",
        "RtlGenRandom from host CSPRNG",
        security::system_function_036
    ),
    export!(
        "advapi32.dll",
        "LookupAccountSidW",
        "security",
        "partial",
        "local user account name and workgroup",
        security::lookup_account_sid_w
    ),
    export!(
        "advapi32.dll",
        "AllocateLocallyUniqueId",
        "security",
        "implemented",
        "unique for the process",
        security::allocate_luid
    ),
    export!(
        "advapi32.dll",
        "GetUserNameA",
        "security",
        "implemented",
        "effective Linux account",
        security::user_name_a
    ),
    export!(
        "advapi32.dll",
        "GetUserNameW",
        "security",
        "implemented",
        "effective Linux account as UTF-16",
        security::user_name_w
    ),
    export!(
        "advapi32.dll",
        "GetSecurityInfo",
        "security",
        "implemented",
        "self-relative descriptor with default owner, group, and empty DACL",
        security::get_security_info
    ),
    export!(
        "advapi32.dll",
        "IsValidSecurityDescriptor",
        "security",
        "implemented",
        "checks revision 1",
        security::is_valid_security_descriptor
    ),
    export!(
        "advapi32.dll",
        "GetSecurityDescriptorDacl",
        "security",
        "implemented",
        "retrieves DACL from security descriptor",
        security::get_security_descriptor_dacl
    ),
    export!(
        "advapi32.dll",
        "GetSecurityDescriptorOwner",
        "security",
        "implemented",
        "retrieves owner from security descriptor",
        security::get_security_descriptor_owner
    ),
    export!(
        "advapi32.dll",
        "GetSecurityDescriptorGroup",
        "security",
        "implemented",
        "retrieves group from security descriptor",
        security::get_security_descriptor_group
    ),
    export!(
        "advapi32.dll",
        "GetSecurityDescriptorLength",
        "security",
        "implemented",
        "fixed length security descriptor",
        security::get_security_descriptor_length
    ),
    export!(
        "advapi32.dll",
        "GetSecurityDescriptorControl",
        "security",
        "implemented",
        "retrieves control flags",
        security::get_security_descriptor_control
    ),
    export!(
        "advapi32.dll",
        "InitializeSecurityDescriptor",
        "security",
        "implemented",
        "initializes security descriptor",
        security::initialize_security_descriptor
    ),
    export!(
        "advapi32.dll",
        "SetSecurityDescriptorDacl",
        "security",
        "implemented",
        "sets DACL on security descriptor",
        security::set_security_descriptor_dacl
    ),
    export!(
        "advapi32.dll",
        "GetAclInformation",
        "security",
        "implemented",
        "retrieves ACL size and ACE count",
        security::get_acl_information
    ),
    export!(
        "advapi32.dll",
        "GetAce",
        "security",
        "implemented",
        "retrieves ACE by index",
        security::get_ace
    ),
    export!(
        "advapi32.dll",
        "IsValidAcl",
        "security",
        "implemented",
        "checks valid ACL",
        security::is_valid_acl
    ),
    export!(
        "advapi32.dll",
        "InitializeAcl",
        "security",
        "implemented",
        "initializes empty ACL",
        security::initialize_acl
    ),
    export!(
        "advapi32.dll",
        "AddAccessAllowedAce",
        "security",
        "implemented",
        "adds access allowed ACE",
        security::add_access_allowed_ace
    ),
    export!(
        "advapi32.dll",
        "AddAccessAllowedAceEx",
        "security",
        "implemented",
        "adds access allowed ACE with flags",
        security::add_access_allowed_ace_ex
    ),
    export!(
        "advapi32.dll",
        "GetSecurityDescriptorSacl",
        "security",
        "implemented",
        "retrieves SACL from security descriptor",
        security::get_security_descriptor_sacl
    ),
    export!(
        "advapi32.dll",
        "SetSecurityDescriptorSacl",
        "security",
        "implemented",
        "sets SACL on security descriptor",
        security::set_security_descriptor_sacl
    ),
    export!(
        "advapi32.dll",
        "SetSecurityDescriptorOwner",
        "security",
        "implemented",
        "sets owner on security descriptor",
        security::set_security_descriptor_owner
    ),
    export!(
        "advapi32.dll",
        "SetSecurityDescriptorGroup",
        "security",
        "implemented",
        "sets group on security descriptor",
        security::set_security_descriptor_group
    ),
    export!(
        "advapi32.dll",
        "MakeAbsoluteSD",
        "security",
        "implemented",
        "converts self-relative security descriptor to absolute",
        security::make_absolute_sd
    ),
    export!(
        "advapi32.dll",
        "MakeSelfRelativeSD",
        "security",
        "implemented",
        "converts absolute security descriptor to self-relative",
        security::make_self_relative_sd
    ),
    export!(
        "advapi32.dll",
        "GetKernelObjectSecurity",
        "security",
        "implemented",
        "returns self-relative security descriptor",
        security::get_kernel_object_security
    ),
    export!(
        "advapi32.dll",
        "SetKernelObjectSecurity",
        "security",
        "partial",
        "accepted without modifying host kernel object",
        security::set_kernel_object_security
    ),
    export!(
        "advapi32.dll",
        "SetSecurityInfo",
        "security",
        "partial",
        "accepted without modifying host object",
        security::set_security_info
    ),
    export!(
        "advapi32.dll",
        "GetNamedSecurityInfoW",
        "security",
        "partial",
        "returns default self-relative security descriptor",
        security::get_named_security_info_w
    ),
    export!(
        "advapi32.dll",
        "SetNamedSecurityInfoW",
        "security",
        "partial",
        "accepted without modifying host object",
        security::set_named_security_info_w
    ),
    export!(
        "advapi32.dll",
        "SetFileSecurityW",
        "security",
        "partial",
        "accepted without modifying host file",
        security::set_file_security_w
    ),
    export!(
        "advapi32.dll",
        "AdjustTokenPrivileges",
        "security",
        "partial",
        "accepted with ERROR_SUCCESS",
        security::adjust_token_privileges
    ),
    export!(
        "advapi32.dll",
        "LookupPrivilegeValueW",
        "security",
        "partial",
        "allocates LUID for privilege",
        security::lookup_privilege_value_w
    ),
    export!(
        "advapi32.dll",
        "LookupPrivilegeValueA",
        "security",
        "partial",
        "allocates LUID for privilege",
        security::lookup_privilege_value_a
    ),
    export!(
        "advapi32.dll",
        "AllocateAndInitializeSid",
        "security",
        "implemented",
        "allocates and initializes SID",
        security::allocate_and_initialize_sid
    ),
    export!(
        "advapi32.dll",
        "FreeSid",
        "security",
        "implemented",
        "releases SID allocated with AllocateAndInitializeSid",
        security::free_sid
    ),
    export!(
        "advapi32.dll",
        "EqualSid",
        "security",
        "implemented",
        "compares two SIDs",
        security::equal_sid
    ),
    export!(
        "advapi32.dll",
        "InitializeSid",
        "security",
        "implemented",
        "initializes SID header",
        security::initialize_sid
    ),
    export!(
        "advapi32.dll",
        "CreateWellKnownSid",
        "security",
        "partial",
        "constructs well-known SID",
        security::create_well_known_sid
    ),
    export!(
        "advapi32.dll",
        "CheckTokenMembership",
        "security",
        "partial",
        "reports membership present",
        security::check_token_membership
    ),
    export!(
        "advapi32.dll",
        "DuplicateToken",
        "security",
        "partial",
        "creates duplicate token handle",
        security::duplicate_token
    ),
    export!(
        "advapi32.dll",
        "DuplicateTokenEx",
        "security",
        "partial",
        "creates duplicate token handle",
        security::duplicate_token_ex
    ),
    export!(
        "advapi32.dll",
        "CreateRestrictedToken",
        "security",
        "partial",
        "creates restricted token handle",
        security::create_restricted_token
    ),
    export!(
        "advapi32.dll",
        "SetTokenInformation",
        "security",
        "partial",
        "accepted without modifying host token",
        security::set_token_information
    ),
    export!(
        "advapi32.dll",
        "SetThreadToken",
        "security",
        "partial",
        "accepted without modifying host thread",
        security::set_thread_token
    ),
    export!(
        "advapi32.dll",
        "RevertToSelf",
        "security",
        "implemented",
        "no-op",
        security::revert_to_self
    ),
    export!(
        "advapi32.dll",
        "ImpersonateLoggedOnUser",
        "security",
        "partial",
        "no-op",
        security::impersonate_logged_on_user
    ),
    export!(
        "advapi32.dll",
        "ConvertSidToStringSidW",
        "security",
        "partial",
        "returns LocalAlloc UTF-16 SID string",
        security::convert_sid_to_string_sid_w
    ),
    export!(
        "advapi32.dll",
        "ConvertSidToStringSidA",
        "security",
        "partial",
        "returns LocalAlloc ASCII SID string",
        security::convert_sid_to_string_sid_a
    ),
    export!(
        "advapi32.dll",
        "ConvertStringSidToSidW",
        "security",
        "partial",
        "returns LocalAlloc SID",
        security::convert_string_sid_to_sid_w
    ),
    export!(
        "advapi32.dll",
        "ConvertStringSecurityDescriptorToSecurityDescriptorW",
        "security",
        "partial",
        "returns LocalAlloc self-relative security descriptor",
        security::convert_string_sd_to_sd_w
    ),

    export!("bcrypt.dll", "BCryptOpenAlgorithmProvider", "crypto", "partial", "RNG, AES, SHA-1/256/384/512, MD5 (HMAC flag); others STATUS_NOT_FOUND", random::bcrypt_open_algorithm_provider),
    export!("bcrypt.dll", "BCryptCloseAlgorithmProvider", "crypto", "implemented", "provider handles", random::bcrypt_close_algorithm_provider),
    export!("bcrypt.dll", "BCryptGetProperty", "crypto", "partial", "object/digest/block/key lengths, ChainingMode, GCM tag lengths; others STATUS_NOT_SUPPORTED", random::bcrypt_get_property),
    export!("bcrypt.dll", "BCryptSetProperty", "crypto", "partial", "ChainingMode CBC/ECB/GCM on AES providers and keys; others STATUS_NOT_SUPPORTED", random::bcrypt_set_property),
    export!("bcrypt.dll", "BCryptCreateHash", "crypto", "implemented", "EVP digest; HMAC with the secret when the provider has the HMAC flag", random::bcrypt_create_hash),
    export!("bcrypt.dll", "BCryptHashData", "crypto", "implemented", "EVP digest update", random::bcrypt_hash_data),
    export!("bcrypt.dll", "BCryptFinishHash", "crypto", "implemented", "EVP digest final", random::bcrypt_finish_hash),
    export!("bcrypt.dll", "BCryptDestroyHash", "crypto", "implemented", "EVP digest context cleanup", random::bcrypt_destroy_hash),
    export!("bcrypt.dll", "BCryptDuplicateHash", "crypto", "implemented", "EVP digest context copy", random::bcrypt_duplicate_hash),
    export!("bcrypt.dll", "BCryptGenerateSymmetricKey", "crypto", "implemented", "EVP symmetric key generation", random::bcrypt_generate_symmetric_key),
    export!("bcrypt.dll", "BCryptDestroyKey", "crypto", "implemented", "EVP symmetric key cleanup", random::bcrypt_destroy_key),
    export!("bcrypt.dll", "BCryptDuplicateKey", "crypto", "implemented", "EVP symmetric key copy", random::bcrypt_duplicate_key),
    export!("bcrypt.dll", "BCryptDecrypt", "crypto", "partial", "AES CBC/ECB (BLOCK_PADDING, IV chaining) and GCM (no chained calls)", random::bcrypt_decrypt),
    export!("bcrypt.dll", "BCryptEncrypt", "crypto", "partial", "AES CBC/ECB (BLOCK_PADDING, IV chaining) and GCM (no chained calls)", random::bcrypt_encrypt),
    export!(
        "bcrypt.dll",
        "BCryptGenRandom",
        "crypto",
        "implemented",
        "validated RNG handle or system-preferred generator, host getrandom",
        random::bcrypt_gen_random
    ),
    export!(
        "bcryptprimitives.dll",
        "ProcessPrng",
        "crypto",
        "implemented",
        "host CSPRNG (getrandom)",
        random::process_prng
    ),
    implemented!(
        "EncodePointer",
        "system",
        "implemented",
        "per-process random cookie and rotation",
        pointers::encode
    ),
    implemented!(
        "DecodePointer",
        "system",
        "implemented",
        "inverse of EncodePointer",
        pointers::decode
    ),
    implemented!(
        "EncodeSystemPointer",
        "system",
        "implemented",
        "same as EncodePointer",
        pointers::encode
    ),
    implemented!(
        "DecodeSystemPointer",
        "system",
        "implemented",
        "same as DecodePointer",
        pointers::decode
    ),
    export!("advapi32.dll", "MapGenericMask", "security", "implemented", "maps generic access rights", security::map_generic_mask),
    export!("advapi32.dll", "AccessCheck", "security", "partial", "grants desired access", security::access_check),
    export!("advapi32.dll", "AddMandatoryAce", "security", "partial", "returns TRUE", security::add_mandatory_ace),
    export!("advapi32.dll", "AddConditionalAce", "security", "partial", "returns TRUE", security::add_conditional_ace),
}
